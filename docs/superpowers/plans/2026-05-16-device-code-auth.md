# Device-Code Authentication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace CookCLI's browser-popup login (broken in Docker per [#290](https://github.com/cooklang/cookcli/issues/290)) with the OAuth 2.0 Device Authorization Grant (RFC 8628), shared by the web UI and a new `cook login` / `cook logout` CLI.

**Architecture:** Two phases across two repos. Phase 1 adds new endpoints to `cook.md` (Rails): `POST /oauth/device/code`, `POST /oauth/device/token`, and a `/device` web page (GET form, POST confirm, POST approve, POST deny). State lives in Redis with 15-minute TTL. Phase 2 adds a shared Rust `device_flow` module to CookCLI, replaces the broken browser-popup flow in the web handler, and adds `cook login` / `cook logout` CLI commands. JWT issuance, session storage, and sync remain unchanged downstream.

**Tech Stack:**
- cook.md: Rails 8, RSpec, RuboCop, `redis` gem (`$redis` global), `rack-attack`, `jwt` gem (via `Token` module).
- CookCLI: Rust, Axum, Askama templates, `reqwest`, `tokio`, `tokio_util::sync::CancellationToken`, `serde_json`, `anyhow`. Existing `sync` cargo feature continues to gate everything.

**Spec:** `docs/superpowers/specs/2026-05-16-device-code-auth-design.md`

---

## File Structure

### Phase 1 — cook.md (`/Users/alexeydubovskoy/Cooklang/cook.md/web`)

**New files:**
- `lib/device_flow.rb` — Redis I/O, code generation, state transitions. One module with class-level helpers.
- `app/controllers/oauth/device_controller.rb` — `POST /oauth/device/code`, `POST /oauth/device/token`. Inherits `ActionController::API` (no CSRF, no Warden).
- `app/controllers/device_controller.rb` — `GET /device`, `POST /device`, `POST /device/approve`, `POST /device/deny`. Inherits `ApplicationController` (Warden web strategy, CSRF on).
- `app/views/device/show.html.erb` — user_code entry form.
- `app/views/device/confirm.html.erb` — approve/deny page.
- `app/views/device/approved.html.erb` — success page.
- `app/views/device/denied.html.erb` — denial confirmation.
- `app/views/device/expired.html.erb` — code expired / invalid.
- `spec/lib/device_flow_spec.rb` — unit specs for `DeviceFlow`.
- `spec/requests/oauth/device_spec.rb` — request specs for `/oauth/device/*`.
- `spec/requests/device_spec.rb` — request specs for the web page.

**Modified:**
- `config/routes.rb` — add 6 new routes.
- `config/initializers/rack_attack.rb` — add 3 throttles.

### Phase 2 — CookCLI (`/Users/alexeydubovskoy/Cooklang/CookCLI`)

**New files:**
- `src/server/sync/device_flow.rs` — `request_device_code()`, `poll_for_token()`, types, errors.
- `src/login.rs` — `cook login` command.
- `src/logout.rs` — `cook logout` command.

**Modified:**
- `src/server/sync/mod.rs` — re-export `device_flow`.
- `src/server/mod.rs` — replace `login_in_progress: AtomicBool` with `pending_device_flow: Mutex<Option<DeviceFlowState>>` in `AppState`; add `/api/sync/cancel_login` route.
- `src/server/handlers/sync.rs` — delete browser-popup helpers; rewrite `sync_login`; extend `LoginResponse`; extend `SyncStatusResponse`; add `sync_cancel_login`.
- `templates/preferences.html` — replace login button JS with the device-code card; status polling reads `pending_login`.
- `src/args.rs` — add `Login(LoginArgs)` and `Logout(LogoutArgs)` variants.
- `src/main.rs` — add match arms.
- `Cargo.toml` — no changes (`open`, `uuid`, `tokio-util`, `reqwest` all already present).

---

# Phase 1 — cook.md (Rails)

Work directory: `/Users/alexeydubovskoy/Cooklang/cook.md/web`. All RSpec / RuboCop commands must be run from there.

### Task 1: Rack::Attack throttles

**Files:**
- Modify: `config/initializers/rack_attack.rb`

- [ ] **Step 1: Read the existing throttles**

Run: `cat config/initializers/rack_attack.rb`

Note the existing pattern (e.g. `auth/code_request/ip` block). New throttles will mirror it.

- [ ] **Step 2: Add three new throttles**

Append to `config/initializers/rack_attack.rb` (inside the `Rack::Attack` configuration block, near the other `throttle` calls):

```ruby
# Device-code issuance: cap creation rate per IP.
throttle("oauth/device/code/ip", limit: 10, period: 1.minute) do |req|
  if req.path == "/oauth/device/code" && req.post?
    Rack::Attack.real_ip(req)
  end
end

# Device-code token polling: hard cap per IP. Per-device_code slow_down is
# enforced inside the controller against last_polled_at.
throttle("oauth/device/token/ip", limit: 60, period: 1.minute) do |req|
  if req.path == "/oauth/device/token" && req.post?
    Rack::Attack.real_ip(req)
  end
end

# /device form submissions: prevent user_code brute-force per session/IP.
throttle("device/submit/ip", limit: 10, period: 15.minutes) do |req|
  if req.path == "/device" && req.post?
    Rack::Attack.real_ip(req)
  end
end
```

- [ ] **Step 3: Verify Rails boots**

Run: `bundle exec rails runner "puts 'ok'"`
Expected: `ok` on stdout, no exceptions.

- [ ] **Step 4: Run rubocop on the file**

Run: `bundle exec rubocop config/initializers/rack_attack.rb`
Expected: no offenses. Fix any styling complaints inline.

- [ ] **Step 5: Commit**

```bash
git add config/initializers/rack_attack.rb
git commit -m "feat(device-flow): add rate limits for device-code endpoints"
```

---

### Task 2: `DeviceFlow` lib module — TDD

The `DeviceFlow` module owns code generation, Redis I/O, and state transitions. Independently testable from any controller.

**Files:**
- Create: `lib/device_flow.rb`
- Create: `spec/lib/device_flow_spec.rb`

- [ ] **Step 1: Write the failing spec**

Create `spec/lib/device_flow_spec.rb`:

```ruby
# frozen_string_literal: true

require "rails_helper"

RSpec.describe DeviceFlow do
  before { $redis.flushdb }

  describe ".create!" do
    it "returns a fresh device_code + user_code" do
      result = described_class.create!(client_name: "CookCLI test")
      expect(result.device_code).to be_a(String).and have_attributes(length: be > 20)
      expect(result.user_code).to match(/\A[23456789ABCDEFGHJKLMNPQRSTUVWXYZ]{4}-[23456789ABCDEFGHJKLMNPQRSTUVWXYZ]{4}\z/)
      expect(result.expires_in).to eq(DeviceFlow::EXPIRES_IN)
      expect(result.interval).to eq(DeviceFlow::DEFAULT_INTERVAL)
    end

    it "persists state as pending with the supplied client_name" do
      result = described_class.create!(client_name: "CookCLI 0.30 (linux/docker)")
      record = described_class.find_by_device_code(result.device_code)
      expect(record).to include("status" => "pending", "client_name" => "CookCLI 0.30 (linux/docker)")
    end

    it "truncates client_name to 80 chars" do
      result = described_class.create!(client_name: "X" * 200)
      record = described_class.find_by_device_code(result.device_code)
      expect(record["client_name"].length).to eq(80)
    end

    it "sets a 15-minute TTL on both keys" do
      result = described_class.create!(client_name: "x")
      device_hash = Digest::SHA256.hexdigest(result.device_code)
      user_code = result.user_code.delete("-")
      expect($redis.ttl("device_code:#{device_hash}")).to be_between(890, 901)
      expect($redis.ttl("user_code:#{user_code}")).to be_between(890, 901)
    end
  end

  describe ".find_by_user_code" do
    it "is hyphen- and case-tolerant" do
      result = described_class.create!(client_name: "x")
      expect(described_class.find_by_user_code(result.user_code)["status"]).to eq("pending")
      expect(described_class.find_by_user_code(result.user_code.delete("-"))["status"]).to eq("pending")
      expect(described_class.find_by_user_code(result.user_code.downcase)["status"]).to eq("pending")
    end

    it "returns nil for an unknown code" do
      expect(described_class.find_by_user_code("AAAA-BBBB")).to be_nil
    end
  end

  describe ".approve!" do
    it "transitions pending → approved, records user_id, deletes user_code index" do
      result = described_class.create!(client_name: "x")
      described_class.approve!(user_code: result.user_code, user_id: 42)
      record = described_class.find_by_device_code(result.device_code)
      expect(record).to include("status" => "approved", "user_id" => 42)
      expect($redis.get("user_code:#{result.user_code.delete('-')}")).to be_nil
    end

    it "returns false for an unknown user_code" do
      expect(described_class.approve!(user_code: "AAAA-BBBB", user_id: 1)).to be(false)
    end
  end

  describe ".deny!" do
    it "transitions pending → denied" do
      result = described_class.create!(client_name: "x")
      described_class.deny!(user_code: result.user_code)
      record = described_class.find_by_device_code(result.device_code)
      expect(record["status"]).to eq("denied")
    end
  end

  describe ".poll" do
    let(:created) { described_class.create!(client_name: "x") }

    it "returns [:pending, nil] before approval" do
      status, payload = described_class.poll(device_code: created.device_code)
      expect(status).to eq(:pending)
      expect(payload).to be_nil
    end

    it "returns [:slow_down, nil] when polled within interval" do
      described_class.poll(device_code: created.device_code)
      status, _ = described_class.poll(device_code: created.device_code)
      expect(status).to eq(:slow_down)
    end

    it "returns [:approved, {user_id}] after approval and then consumes the code" do
      described_class.approve!(user_code: created.user_code, user_id: 7)
      status, payload = described_class.poll(device_code: created.device_code)
      expect(status).to eq(:approved)
      expect(payload).to eq(user_id: 7)
      # Subsequent poll: code is consumed
      status2, _ = described_class.poll(device_code: created.device_code)
      expect(status2).to eq(:expired)
    end

    it "returns [:denied, nil] after deny" do
      described_class.deny!(user_code: created.user_code)
      status, _ = described_class.poll(device_code: created.device_code)
      expect(status).to eq(:denied)
    end

    it "returns [:expired, nil] for an unknown device_code" do
      status, _ = described_class.poll(device_code: "bogus")
      expect(status).to eq(:expired)
    end
  end
end
```

- [ ] **Step 2: Run spec to verify it fails**

Run: `bundle exec rspec spec/lib/device_flow_spec.rb`
Expected: All examples error with `uninitialized constant DeviceFlow` (file not yet created).

- [ ] **Step 3: Implement the module**

Create `lib/device_flow.rb`:

```ruby
# frozen_string_literal: true

require "digest"
require "securerandom"

module DeviceFlow
  EXPIRES_IN = 900  # 15 minutes (seconds)
  DEFAULT_INTERVAL = 5
  SLOW_DOWN_BUMP = 5
  USER_CODE_ALPHABET = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ".freeze
  USER_CODE_LEN = 8
  CLIENT_NAME_MAX = 80

  Created = Struct.new(:device_code, :user_code, :expires_in, :interval, keyword_init: true)

  def self.create!(client_name:)
    device_code = SecureRandom.urlsafe_base64(32)
    user_code = generate_user_code
    hash = Digest::SHA256.hexdigest(device_code)
    safe_name = (client_name || "Unknown client").to_s[0, CLIENT_NAME_MAX]

    record = {
      "user_code" => user_code,
      "client_name" => safe_name,
      "status" => "pending",
      "user_id" => nil,
      "last_polled_at" => nil
    }

    $redis.set("device_code:#{hash}", record.to_json, ex: EXPIRES_IN)
    $redis.set("user_code:#{user_code}", hash, ex: EXPIRES_IN)

    Created.new(
      device_code: device_code,
      user_code: format_user_code(user_code),
      expires_in: EXPIRES_IN,
      interval: DEFAULT_INTERVAL
    )
  end

  def self.find_by_device_code(device_code)
    hash = Digest::SHA256.hexdigest(device_code)
    raw = $redis.get("device_code:#{hash}")
    raw && JSON.parse(raw)
  end

  def self.find_by_user_code(user_code)
    normalized = normalize_user_code(user_code)
    hash = $redis.get("user_code:#{normalized}")
    return nil unless hash

    raw = $redis.get("device_code:#{hash}")
    raw && JSON.parse(raw)
  end

  def self.approve!(user_code:, user_id:)
    update_state(user_code: user_code, status: "approved", user_id: user_id)
  end

  def self.deny!(user_code:)
    update_state(user_code: user_code, status: "denied", user_id: nil)
  end

  # Returns [status_symbol, payload_or_nil].
  # status_symbol ∈ [:pending, :slow_down, :approved, :denied, :expired]
  # On :approved, payload is { user_id: Integer }. The Redis entry is deleted
  # on the same call (single-use).
  def self.poll(device_code:)
    hash = Digest::SHA256.hexdigest(device_code)
    raw = $redis.get("device_code:#{hash}")
    return [:expired, nil] unless raw

    record = JSON.parse(raw)
    now = Time.now.to_i

    case record["status"]
    when "approved"
      payload = { user_id: record["user_id"] }
      $redis.del("device_code:#{hash}")
      [:approved, payload]
    when "denied"
      [:denied, nil]
    else
      # pending
      last = record["last_polled_at"]
      if last && (now - last) < DEFAULT_INTERVAL
        [:slow_down, nil]
      else
        record["last_polled_at"] = now
        ttl = $redis.ttl("device_code:#{hash}")
        $redis.set("device_code:#{hash}", record.to_json, ex: ttl.positive? ? ttl : EXPIRES_IN)
        [:pending, nil]
      end
    end
  end

  def self.normalize_user_code(code)
    code.to_s.upcase.delete("-")
  end

  class << self
    private

    def generate_user_code
      Array.new(USER_CODE_LEN) { USER_CODE_ALPHABET[SecureRandom.random_number(USER_CODE_ALPHABET.length)] }.join
    end

    def format_user_code(raw)
      "#{raw[0, 4]}-#{raw[4, 4]}"
    end

    def update_state(user_code:, status:, user_id:)
      normalized = normalize_user_code(user_code)
      hash = $redis.get("user_code:#{normalized}")
      return false unless hash

      raw = $redis.get("device_code:#{hash}")
      return false unless raw

      record = JSON.parse(raw)
      record["status"] = status
      record["user_id"] = user_id if user_id

      ttl = $redis.ttl("device_code:#{hash}")
      $redis.set("device_code:#{hash}", record.to_json, ex: ttl.positive? ? ttl : EXPIRES_IN)
      $redis.del("user_code:#{normalized}")
      true
    end
  end
end
```

- [ ] **Step 4: Run spec to verify it passes**

Run: `bundle exec rspec spec/lib/device_flow_spec.rb`
Expected: all examples pass.

- [ ] **Step 5: RuboCop**

Run: `bundle exec rubocop lib/device_flow.rb spec/lib/device_flow_spec.rb`
Expected: no offenses. Fix inline if any.

- [ ] **Step 6: Commit**

```bash
git add lib/device_flow.rb spec/lib/device_flow_spec.rb
git commit -m "feat(device-flow): add DeviceFlow lib module with Redis state"
```

---

### Task 3: `POST /oauth/device/code` endpoint — TDD

**Files:**
- Create: `app/controllers/oauth/device_controller.rb`
- Create: `spec/requests/oauth/device_spec.rb`
- Modify: `config/routes.rb`

- [ ] **Step 1: Write the failing request spec for `/oauth/device/code`**

Create `spec/requests/oauth/device_spec.rb`:

```ruby
# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Oauth::Device", type: :request do
  before { $redis.flushdb }

  describe "POST /oauth/device/code" do
    it "returns 200 with a complete RFC 8628 response body" do
      post "/oauth/device/code", params: { client_name: "CookCLI 0.30 (linux)" }, as: :json
      expect(response).to have_http_status(:ok)
      body = response.parsed_body
      expect(body["device_code"]).to be_a(String)
      expect(body["user_code"]).to match(/\A[A-Z0-9]{4}-[A-Z0-9]{4}\z/)
      expect(body["verification_uri"]).to end_with("/device")
      expect(body["verification_uri_complete"]).to include("user_code=#{body['user_code']}")
      expect(body["expires_in"]).to eq(900)
      expect(body["interval"]).to eq(5)
    end

    it "accepts a missing client_name and uses a default" do
      post "/oauth/device/code", as: :json
      expect(response).to have_http_status(:ok)
    end
  end
end
```

- [ ] **Step 2: Run spec to verify it fails**

Run: `bundle exec rspec spec/requests/oauth/device_spec.rb`
Expected: routing error / 404 — route does not exist yet.

- [ ] **Step 3: Add the route**

Modify `config/routes.rb`. Find the `namespace :auth do ... end` block (around line 114). Immediately *after* that block (before `resource :profile`), add:

```ruby
namespace :oauth do
  post "device/code", to: "device#code"
  post "device/token", to: "device#token"
end
```

- [ ] **Step 4: Implement the controller**

Create `app/controllers/oauth/device_controller.rb`:

```ruby
# frozen_string_literal: true

module Oauth
  class DeviceController < ActionController::API
    def code
      result = DeviceFlow.create!(client_name: params[:client_name])
      verification_uri = url_for(controller: "/device", action: "show", only_path: false)
      verification_uri_complete = url_for(
        controller: "/device", action: "show", only_path: false,
        user_code: result.user_code
      )

      render json: {
        device_code: result.device_code,
        user_code: result.user_code,
        verification_uri: verification_uri,
        verification_uri_complete: verification_uri_complete,
        expires_in: result.expires_in,
        interval: result.interval
      }
    end

    def token
      # Placeholder — implemented in Task 4.
      head :not_implemented
    end
  end
end
```

> Note: `url_for(controller: "/device", action: "show", ...)` resolves to the top-level `device#show` route added in Task 5. Until that route exists Rails will raise. The next step adds a stub route so this controller compiles end-to-end immediately; Task 5 replaces the stub with the real view-rendering action.

- [ ] **Step 5: Add a placeholder `/device` route so URL generation works**

Modify `config/routes.rb`. Anywhere outside the `namespace :auth` and `namespace :oauth` blocks, add:

```ruby
get "/device", to: "device#show", as: :device
```

You also need a stub controller so route loading doesn't error during Task 3 specs. Create `app/controllers/device_controller.rb`:

```ruby
# frozen_string_literal: true

class DeviceController < ApplicationController
  def show
    render plain: "stub", status: :ok
  end
end
```

(Task 5 replaces both action and view with the real ones.)

- [ ] **Step 6: Run spec to verify it passes**

Run: `bundle exec rspec spec/requests/oauth/device_spec.rb`
Expected: both examples pass.

- [ ] **Step 7: RuboCop**

Run: `bundle exec rubocop app/controllers/oauth/device_controller.rb app/controllers/device_controller.rb spec/requests/oauth/device_spec.rb config/routes.rb`
Expected: no offenses.

- [ ] **Step 8: Commit**

```bash
git add app/controllers/oauth/device_controller.rb app/controllers/device_controller.rb \
        spec/requests/oauth/device_spec.rb config/routes.rb
git commit -m "feat(device-flow): add POST /oauth/device/code endpoint"
```

---

### Task 4: `POST /oauth/device/token` endpoint — TDD

Implements the full RFC 8628 token-polling state machine.

**Files:**
- Modify: `app/controllers/oauth/device_controller.rb`
- Modify: `spec/requests/oauth/device_spec.rb`

- [ ] **Step 1: Extend the spec**

Append inside `RSpec.describe "Oauth::Device", type: :request do` in `spec/requests/oauth/device_spec.rb`:

```ruby
  describe "POST /oauth/device/token" do
    let(:created) { DeviceFlow.create!(client_name: "x") }
    let(:grant) { "urn:ietf:params:oauth:grant-type:device_code" }
    let(:user) { User.create!(email: "alice@example.com") }

    def poll(device_code:, grant_type: grant)
      post "/oauth/device/token", params: { device_code: device_code, grant_type: grant_type }, as: :json
    end

    it "returns 400 authorization_pending while still pending" do
      poll(device_code: created.device_code)
      expect(response).to have_http_status(:bad_request)
      expect(response.parsed_body["error"]).to eq("authorization_pending")
    end

    it "returns 400 slow_down on rapid re-poll" do
      poll(device_code: created.device_code)
      poll(device_code: created.device_code)
      expect(response).to have_http_status(:bad_request)
      expect(response.parsed_body["error"]).to eq("slow_down")
    end

    it "returns 200 with a JWT on approval, then 400 expired_token on replay" do
      DeviceFlow.approve!(user_code: created.user_code, user_id: user.id)
      poll(device_code: created.device_code)
      expect(response).to have_http_status(:ok)
      body = response.parsed_body
      expect(body["token"]).to be_a(String)
      expect(body["token_type"]).to eq("Bearer")
      decoded = Token.decode(body["token"])
      expect(decoded["uid"]).to eq(user.id)
      expect(decoded["email"]).to eq(user.email)

      # Replay
      poll(device_code: created.device_code)
      expect(response).to have_http_status(:bad_request)
      expect(response.parsed_body["error"]).to eq("expired_token")
    end

    it "returns 400 access_denied after deny" do
      DeviceFlow.deny!(user_code: created.user_code)
      poll(device_code: created.device_code)
      expect(response).to have_http_status(:bad_request)
      expect(response.parsed_body["error"]).to eq("access_denied")
    end

    it "returns 400 expired_token for an unknown device_code" do
      poll(device_code: "bogus")
      expect(response).to have_http_status(:bad_request)
      expect(response.parsed_body["error"]).to eq("expired_token")
    end

    it "returns 400 invalid_grant on wrong grant_type" do
      poll(device_code: created.device_code, grant_type: "wrong")
      expect(response).to have_http_status(:bad_request)
      expect(response.parsed_body["error"]).to eq("invalid_grant")
    end
  end
```

- [ ] **Step 2: Run spec to verify it fails**

Run: `bundle exec rspec spec/requests/oauth/device_spec.rb`
Expected: the `token` examples fail (the action returns 501).

- [ ] **Step 3: Implement `token` action**

Replace the `token` method in `app/controllers/oauth/device_controller.rb` with:

```ruby
    DEVICE_CODE_GRANT = "urn:ietf:params:oauth:grant-type:device_code"

    def token
      if params[:grant_type] != DEVICE_CODE_GRANT
        return render(json: { error: "invalid_grant" }, status: :bad_request)
      end

      status, payload = DeviceFlow.poll(device_code: params[:device_code].to_s)

      case status
      when :pending
        render json: { error: "authorization_pending" }, status: :bad_request
      when :slow_down
        render json: { error: "slow_down" }, status: :bad_request
      when :denied
        render json: { error: "access_denied" }, status: :bad_request
      when :expired
        render json: { error: "expired_token" }, status: :bad_request
      when :approved
        user = User.find_by(id: payload[:user_id])
        if user.nil?
          render json: { error: "access_denied" }, status: :bad_request
        else
          jwt = Token.encode(uid: user.id, email: user.email)
          render json: { token: jwt, token_type: "Bearer" }
        end
      end
    end
```

- [ ] **Step 4: Run spec to verify it passes**

Run: `bundle exec rspec spec/requests/oauth/device_spec.rb`
Expected: all examples pass.

- [ ] **Step 5: RuboCop**

Run: `bundle exec rubocop app/controllers/oauth/device_controller.rb spec/requests/oauth/device_spec.rb`
Expected: no offenses.

- [ ] **Step 6: Commit**

```bash
git add app/controllers/oauth/device_controller.rb spec/requests/oauth/device_spec.rb
git commit -m "feat(device-flow): implement POST /oauth/device/token state machine"
```

---

### Task 5: `GET /device` form & `POST /device` confirm — TDD

**Files:**
- Modify: `config/routes.rb`
- Modify: `app/controllers/device_controller.rb`
- Create: `app/views/device/show.html.erb`
- Create: `app/views/device/confirm.html.erb`
- Create: `spec/requests/device_spec.rb`

- [ ] **Step 1: Write the failing request spec**

Create `spec/requests/device_spec.rb`:

```ruby
# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Device", type: :request do
  before { $redis.flushdb }

  let(:user) { User.create!(email: "alice@example.com") }

  def sign_in_as(u)
    # Mirror the test helper used elsewhere for Warden web strategy login.
    # If a helper like `login_as` exists in spec/support, use that instead.
    post "/auth/desktop/code", params: { email: u.email }
    # In tests, SessionCode for test@cook.md returns 424242; for other emails
    # the code is generated. Inject directly via SessionCode if necessary.
    code = $redis.get("code:#{u.email}")
    post "/auth/desktop/code/verify_code", params: { email: u.email, code: code }
  end

  describe "GET /device" do
    context "when signed out" do
      it "redirects to sign-in, preserving user_code via session" do
        get "/device", params: { user_code: "ABCD-EFGH" }
        expect(response).to redirect_to(sign_in_path)
      end
    end

    context "when signed in" do
      before { sign_in_as(user) }

      it "renders the form" do
        get "/device"
        expect(response).to have_http_status(:ok)
        expect(response.body).to include("Enter the code shown by CookCLI")
      end

      it "pre-fills the input from ?user_code=" do
        get "/device", params: { user_code: "WDJB-MJHT" }
        expect(response.body).to include('value="WDJB-MJHT"')
      end
    end
  end

  describe "POST /device" do
    before { sign_in_as(user) }

    it "renders the confirm page for a known user_code" do
      created = DeviceFlow.create!(client_name: "CookCLI test")
      post "/device", params: { user_code: created.user_code }
      expect(response).to have_http_status(:ok)
      expect(response.body).to include("CookCLI test")
      expect(response.body).to include(user.email)
      expect(response.body).to include("Approve")
      expect(response.body).to include("Deny")
    end

    it "renders the expired view for an unknown user_code" do
      post "/device", params: { user_code: "ZZZZ-ZZZZ" }
      expect(response).to have_http_status(:ok)
      expect(response.body).to include("expired or invalid")
    end
  end
end
```

> Note: the `sign_in_as` helper above depends on the email-code flow already wired in the app. If `spec/support` provides a faster login helper (e.g. via Warden test mode), prefer it. Look in `spec/rails_helper.rb` and `spec/support/`. Adjust as needed but keep the assertions identical.

- [ ] **Step 2: Run spec to verify it fails**

Run: `bundle exec rspec spec/requests/device_spec.rb`
Expected: examples fail (controller still returns the "stub" string; views don't exist).

- [ ] **Step 3: Replace the stub route + add the new ones**

In `config/routes.rb`, replace the line added in Task 3:

```ruby
get "/device", to: "device#show", as: :device
```

with:

```ruby
get  "/device", to: "device#show", as: :device
post "/device", to: "device#confirm"
post "/device/approve", to: "device#approve", as: :device_approve
post "/device/deny",    to: "device#deny",    as: :device_deny
```

- [ ] **Step 4: Implement the controller actions**

Replace `app/controllers/device_controller.rb` with:

```ruby
# frozen_string_literal: true

class DeviceController < ApplicationController
  before_action :authenticate!

  # GET /device
  def show
    @user_code = params[:user_code].to_s.upcase
  end

  # POST /device — looks up the user_code and renders approve/deny page.
  def confirm
    @user_code = params[:user_code].to_s
    @record = DeviceFlow.find_by_user_code(@user_code)
    if @record.nil? || @record["status"] != "pending"
      render :expired
    else
      @client_name = @record["client_name"]
      @email = current_user.email
      render :confirm
    end
  end

  # POST /device/approve
  def approve
    if DeviceFlow.approve!(user_code: params[:user_code].to_s, user_id: current_user.id)
      render :approved
    else
      render :expired
    end
  end

  # POST /device/deny
  def deny
    if DeviceFlow.deny!(user_code: params[:user_code].to_s)
      render :denied
    else
      render :expired
    end
  end
end
```

> The `authenticate!` before-action (defined in `ApplicationController`) redirects to `sign_in_path` for signed-out users. Devise/Warden flash-based return-URL preservation is already handled by the existing sign-in flow.

> If the existing sign-in flow does **not** auto-return to `/device` after login, the spec's first `it` block ("redirects to sign-in") will pass but the subsequent user journey requires the user to navigate back manually with `?user_code=`. That's an acceptable v1 trade-off and matches RFC 8628; document it in the implementation plan completion notes.

- [ ] **Step 5: Implement the views**

Create `app/views/device/show.html.erb`:

```erb
<% content_for :title, "Enter Device Code" %>
<div class="flex flex-col h-screen">
  <div class="flex-1 bg-neutral-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
    <div class="sm:mx-auto sm:w-full sm:max-w-md">
      <h2 class="mt-6 text-center text-3xl font-semibold text-cook-text">
        Enter the code shown by CookCLI
      </h2>
      <p class="mt-2 text-center text-sm text-cook-text/60">
        Signed in as <%= current_user.email %>.
      </p>
    </div>

    <div class="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
      <div class="bg-white py-8 px-4 shadow-sm border border-cook-text/10 sm:rounded-lg sm:px-10">
        <%= form_with url: device_path, method: :post, data: { turbo: false }, local: true do |f| %>
          <%= f.label :user_code, "One-time code", class: "block text-sm font-medium text-cook-text" %>
          <%= f.text_field :user_code,
                value: @user_code,
                autofocus: true,
                required: true,
                pattern: "[A-Za-z0-9-]{8,9}",
                class: "mt-1 block w-full px-3 py-2 border border-cook-text/20 rounded-md uppercase tracking-widest text-center text-xl" %>
          <%= f.submit "Continue", class: "mt-4 w-full inline-flex justify-center py-2 px-4 border rounded-md bg-cook-text text-white" %>
        <% end %>
      </div>
    </div>
  </div>
</div>
```

Create `app/views/device/confirm.html.erb`:

```erb
<% content_for :title, "Approve CookCLI" %>
<div class="flex flex-col h-screen">
  <div class="flex-1 bg-neutral-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
    <div class="sm:mx-auto sm:w-full sm:max-w-md">
      <h2 class="mt-6 text-center text-3xl font-semibold text-cook-text">
        Authorize this device?
      </h2>
      <p class="mt-2 text-center text-sm text-cook-text/70">
        <strong><%= @client_name %></strong> wants to access your
        CookCloud account as <strong><%= @email %></strong>.
      </p>
    </div>

    <div class="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
      <div class="bg-white py-8 px-4 shadow-sm border border-cook-text/10 sm:rounded-lg sm:px-10 flex gap-3">
        <%= form_with url: device_approve_path, method: :post, data: { turbo: false }, local: true, class: "flex-1" do |f| %>
          <%= f.hidden_field :user_code, value: @user_code %>
          <%= f.submit "Approve", class: "w-full py-2 px-4 rounded-md bg-cook-text text-white" %>
        <% end %>
        <%= form_with url: device_deny_path, method: :post, data: { turbo: false }, local: true, class: "flex-1" do |f| %>
          <%= f.hidden_field :user_code, value: @user_code %>
          <%= f.submit "Deny", class: "w-full py-2 px-4 rounded-md border border-red-300 text-red-600" %>
        <% end %>
      </div>
    </div>
  </div>
</div>
```

- [ ] **Step 6: Run spec to verify it passes**

Run: `bundle exec rspec spec/requests/device_spec.rb`
Expected: all `GET /device` and `POST /device` examples pass.

- [ ] **Step 7: RuboCop**

Run: `bundle exec rubocop app/controllers/device_controller.rb spec/requests/device_spec.rb`
Expected: no offenses.

- [ ] **Step 8: Commit**

```bash
git add config/routes.rb app/controllers/device_controller.rb \
        app/views/device/show.html.erb app/views/device/confirm.html.erb \
        spec/requests/device_spec.rb
git commit -m "feat(device-flow): add /device form and confirmation page"
```

---

### Task 6: `POST /device/approve` & `/device/deny` end-to-end — TDD

Adds remaining specs covering the action endpoints and creates the result views. The controller actions were already added in Task 5.

**Files:**
- Modify: `spec/requests/device_spec.rb`
- Create: `app/views/device/approved.html.erb`
- Create: `app/views/device/denied.html.erb`
- Create: `app/views/device/expired.html.erb`

- [ ] **Step 1: Append specs**

Append inside `RSpec.describe "Device", type: :request do` in `spec/requests/device_spec.rb`:

```ruby
  describe "POST /device/approve" do
    before { sign_in_as(user) }

    it "marks the device approved and renders the approved view" do
      created = DeviceFlow.create!(client_name: "x")
      post "/device/approve", params: { user_code: created.user_code }
      expect(response).to have_http_status(:ok)
      expect(response.body).to include("All done")
      expect(DeviceFlow.find_by_device_code(created.device_code)["status"]).to eq("approved")
    end

    it "renders expired for an unknown user_code" do
      post "/device/approve", params: { user_code: "ZZZZ-ZZZZ" }
      expect(response.body).to include("expired or invalid")
    end
  end

  describe "POST /device/deny" do
    before { sign_in_as(user) }

    it "marks the device denied and renders the denied view" do
      created = DeviceFlow.create!(client_name: "x")
      post "/device/deny", params: { user_code: created.user_code }
      expect(response).to have_http_status(:ok)
      expect(response.body).to include("Authorization denied")
      expect(DeviceFlow.find_by_device_code(created.device_code)["status"]).to eq("denied")
    end
  end
```

- [ ] **Step 2: Run spec to verify it fails**

Run: `bundle exec rspec spec/requests/device_spec.rb`
Expected: examples fail with `ActionView::MissingTemplate` for `approved`, `denied`, `expired`.

- [ ] **Step 3: Create the three result views**

Create `app/views/device/approved.html.erb`:

```erb
<% content_for :title, "Authorized" %>
<div class="flex flex-col h-screen">
  <div class="flex-1 bg-neutral-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
    <div class="sm:mx-auto sm:w-full sm:max-w-md text-center">
      <h2 class="mt-6 text-3xl font-semibold text-cook-text">All done</h2>
      <p class="mt-2 text-cook-text/70">You can close this tab and return to CookCLI.</p>
    </div>
  </div>
</div>
```

Create `app/views/device/denied.html.erb`:

```erb
<% content_for :title, "Authorization Denied" %>
<div class="flex flex-col h-screen">
  <div class="flex-1 bg-neutral-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
    <div class="sm:mx-auto sm:w-full sm:max-w-md text-center">
      <h2 class="mt-6 text-3xl font-semibold text-cook-text">Authorization denied</h2>
      <p class="mt-2 text-cook-text/70">CookCLI will not be granted access.</p>
    </div>
  </div>
</div>
```

Create `app/views/device/expired.html.erb`:

```erb
<% content_for :title, "Code Expired" %>
<div class="flex flex-col h-screen">
  <div class="flex-1 bg-neutral-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
    <div class="sm:mx-auto sm:w-full sm:max-w-md text-center">
      <h2 class="mt-6 text-3xl font-semibold text-cook-text">Code expired or invalid</h2>
      <p class="mt-2 text-cook-text/70">Go back to CookCLI and start a new sign-in.</p>
      <p class="mt-6"><%= link_to "Try again", device_path, class: "underline" %></p>
    </div>
  </div>
</div>
```

- [ ] **Step 4: Run full Phase-1 specs**

Run: `bundle exec rspec spec/lib/device_flow_spec.rb spec/requests/oauth/device_spec.rb spec/requests/device_spec.rb`
Expected: all examples pass.

- [ ] **Step 5: RuboCop on the touched files**

Run: `bundle exec rubocop spec/requests/device_spec.rb app/views/device/`
Expected: no offenses.

- [ ] **Step 6: Commit**

```bash
git add spec/requests/device_spec.rb \
        app/views/device/approved.html.erb \
        app/views/device/denied.html.erb \
        app/views/device/expired.html.erb
git commit -m "feat(device-flow): add approve/deny actions and result views"
```

---

### Task 7: Phase-1 verification

- [ ] **Step 1: Run full suite**

Run: `bundle exec rspec`
Expected: no new failures; all Phase-1 device-flow specs pass. If any unrelated specs fail in CI but not locally, document and proceed — do not modify them.

- [ ] **Step 2: Run full RuboCop**

Run: `bundle exec rubocop`
Expected: no offenses (or no *new* offenses vs main if the repo has a baseline file).

- [ ] **Step 3: Manual smoke test**

Start the dev server. In one terminal:
```
bundle exec rails server
```

In another, exercise the API:
```
# 1. Request a code
curl -sS -X POST http://localhost:3000/oauth/device/code \
     -H 'Content-Type: application/json' \
     -d '{"client_name":"smoke test"}' | tee /tmp/device.json

# 2. Poll — expect authorization_pending
curl -sS -X POST http://localhost:3000/oauth/device/token \
     -H 'Content-Type: application/json' \
     -d "{\"grant_type\":\"urn:ietf:params:oauth:grant-type:device_code\",\"device_code\":\"$(jq -r .device_code /tmp/device.json)\"}"

# 3. In a browser, go to http://localhost:3000/device, sign in if needed,
#    enter the user_code, approve.

# 4. Poll again — expect 200 with token
```

Confirm the returned JWT decodes (`Token.decode` in `rails console`).

- [ ] **Step 4: Final Phase-1 commit (if anything changed)**

If `rubocop --auto-correct` or any cleanup was needed, commit it now with `chore(device-flow): tidy Phase-1`.

---

# Phase 2 — CookCLI (Rust)

Work directory: `/Users/alexeydubovskoy/Cooklang/CookCLI`. Run `cargo` commands from there.

> **Note:** the project has no Rust test harness today (per `CONTRIBUTING.md`). Each task verifies via `cargo build --features sync`, `cargo fmt`, `cargo clippy --features sync -- -D warnings`, and explicit manual steps. Do not introduce a test framework as part of this plan.

> **Note:** Phase 2 assumes Phase 1 is deployed (or running locally). For local end-to-end testing, set `COOK_ENDPOINT=http://localhost:3000` before running CookCLI.

### Task 8: New `device_flow.rs` shared module

**Files:**
- Create: `src/server/sync/device_flow.rs`
- Modify: `src/server/sync/mod.rs`

- [ ] **Step 1: Read existing sync module structure**

Run: `cat src/server/sync/mod.rs | head -30`

Identify the existing `pub use` lines and where to add a new one.

- [ ] **Step 2: Create the device_flow module**

Create `src/server/sync/device_flow.rs`:

```rust
use std::time::{Duration, Instant};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::endpoints;

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
struct DeviceCodeRequest<'a> {
    client_name: &'a str,
}

#[derive(Debug, Serialize)]
struct TokenRequest<'a> {
    grant_type: &'a str,
    device_code: &'a str,
}

#[derive(Debug, Deserialize)]
struct TokenSuccess {
    token: String,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceFlowError {
    #[error("user denied authorization")]
    AccessDenied,
    #[error("device code expired")]
    Expired,
    #[error("flow cancelled")]
    Cancelled,
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("bad response from cook.md: {0}")]
    BadResponse(String),
}

pub async fn request_device_code(
    client: &reqwest::Client,
    client_name: &str,
) -> anyhow::Result<DeviceCodeResponse> {
    let url = format!("{}/oauth/device/code", endpoints::base_url());
    let resp = client
        .post(&url)
        .json(&DeviceCodeRequest { client_name })
        .send()
        .await
        .context("calling /oauth/device/code")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("device code request failed: HTTP {status}: {body}");
    }

    resp.json::<DeviceCodeResponse>()
        .await
        .context("parsing device code response")
}

/// Polls /oauth/device/token until approved, denied, expired, or cancelled.
/// Respects `slow_down` (bumps interval by 5 s) and the `expires_at` deadline.
pub async fn poll_for_token(
    client: &reqwest::Client,
    device_code: &str,
    mut interval: Duration,
    expires_at: Instant,
    cancel: CancellationToken,
) -> Result<String, DeviceFlowError> {
    let url = format!("{}/oauth/device/token", endpoints::base_url());

    loop {
        if Instant::now() >= expires_at {
            return Err(DeviceFlowError::Expired);
        }

        tokio::select! {
            _ = cancel.cancelled() => return Err(DeviceFlowError::Cancelled),
            _ = tokio::time::sleep(interval) => {}
        }

        let resp = client
            .post(&url)
            .json(&TokenRequest { grant_type: GRANT_TYPE, device_code })
            .send()
            .await?;

        let status = resp.status();

        if status.is_success() {
            let body: TokenSuccess = resp.json().await.map_err(DeviceFlowError::Network)?;
            return Ok(body.token);
        }

        // 400 → parse {"error": "..."} per RFC 8628
        let body: TokenError = resp
            .json()
            .await
            .map_err(|e| DeviceFlowError::BadResponse(format!("unparseable error body: {e}")))?;

        match body.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                interval += Duration::from_secs(5);
            }
            "access_denied" => return Err(DeviceFlowError::AccessDenied),
            "expired_token" => return Err(DeviceFlowError::Expired),
            other => {
                return Err(DeviceFlowError::BadResponse(format!(
                    "unexpected error code: {other}"
                )))
            }
        }
    }
}

/// Builds the client_name string sent to cook.md. Includes OS and a
/// best-effort label ("docker" / "cli" / hostname).
pub fn client_name(suffix: &str) -> String {
    format!(
        "CookCLI {} ({}/{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        suffix
    )
}

/// Returns "docker" if /.dockerenv exists, else "server".
pub fn server_host_label() -> &'static str {
    if std::path::Path::new("/.dockerenv").exists() {
        "docker"
    } else {
        "server"
    }
}
```

- [ ] **Step 3: Re-export from `sync/mod.rs`**

In `src/server/sync/mod.rs`, find the existing module declarations (likely a block of `pub mod ...` or `mod ...`). Add:

```rust
pub mod device_flow;
```

Add this alongside the other `mod` lines, keeping them grouped.

- [ ] **Step 4: Confirm `thiserror` is available**

Run: `grep '^thiserror' Cargo.toml`

Expected: a dependency line. If missing, add `thiserror = "2"` to `[dependencies]`. (Most Rust projects already have it; the existing `cooklang-sync-client` likely transitively brings it.)

If `thiserror` is *not* a direct dependency, **either** add it or change `DeviceFlowError` to use a manual `impl std::error::Error`. Adding the dep is simpler and ~zero binary cost.

- [ ] **Step 5: Build with the sync feature**

Run: `cargo build --features sync`
Expected: clean build. Fix any compile errors.

- [ ] **Step 6: Lint**

Run: `cargo fmt --check && cargo clippy --features sync -- -D warnings`
Expected: no output, exit 0.

- [ ] **Step 7: Commit**

```bash
git add src/server/sync/device_flow.rs src/server/sync/mod.rs Cargo.toml Cargo.lock
git commit -m "feat(sync): add device_flow module for OAuth device-code login"
```

---

### Task 9: Update `AppState` — replace `login_in_progress` with `pending_device_flow`

**Files:**
- Modify: `src/server/mod.rs`

- [ ] **Step 1: Read the relevant block**

Run: `sed -n '355,400p' src/server/mod.rs`

Locate the `AppState` struct definition.

- [ ] **Step 2: Replace the `login_in_progress` field**

In `src/server/mod.rs`, find the field:

```rust
    #[cfg(feature = "sync")]
    pub login_in_progress: Arc<AtomicBool>,
```

Replace it with:

```rust
    #[cfg(feature = "sync")]
    pub pending_device_flow: Arc<tokio::sync::Mutex<Option<sync::PendingDeviceFlow>>>,
```

- [ ] **Step 3: Add the `PendingDeviceFlow` type to the sync module**

In `src/server/sync/mod.rs`, add:

```rust
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct PendingDeviceFlow {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_at: std::time::Instant,
    pub interval: std::time::Duration,
    pub cancel: CancellationToken,
}
```

- [ ] **Step 4: Update `build_state` initialization**

In `src/server/mod.rs`, find the `build_state` function and locate where `login_in_progress` is initialized:

```rust
        #[cfg(feature = "sync")]
        login_in_progress: Arc::new(AtomicBool::new(false)),
```

Replace with:

```rust
        #[cfg(feature = "sync")]
        pending_device_flow: Arc::new(tokio::sync::Mutex::new(None)),
```

- [ ] **Step 5: Remove the now-unused `AtomicBool` import if it isn't used elsewhere**

Run: `grep -n AtomicBool src/server/mod.rs`

If `AtomicBool` is only referenced by the line above (now deleted), remove its `use` line. Otherwise leave it.

- [ ] **Step 6: Build**

Run: `cargo build --features sync`
Expected: build will fail with errors in `src/server/handlers/sync.rs` (still references `login_in_progress`). That's expected — Task 10 rewrites that file.

To unblock the build for this isolated commit, temporarily comment out (or `todo!()`) the `login_in_progress` references in `handlers/sync.rs`. **Do not delete them yet** — Task 10 deletes the entire helper graph.

Actually simplest: keep the handler unchanged in this task and do Steps 1-5 together with Task 10 as a single commit. Skip ahead and combine — see Task 10.

- [ ] **Step 7: Combine with Task 10**

This task's commit is folded into Task 10 (which rewrites the handler). Do not commit yet.

---

### Task 10: Rewrite `handlers/sync.rs` for device-code flow

**Files:**
- Modify: `src/server/handlers/sync.rs`
- Modify: `src/server/mod.rs` (apply Task 9's changes here too)

This is the largest single change in Phase 2. It rewrites `sync_login`, deletes the browser-popup helpers, extends `LoginResponse` and `SyncStatusResponse`, and adds `sync_cancel_login`.

- [ ] **Step 1: Apply Task 9 changes**

If not already done, apply Task 9 Steps 2-5 now.

- [ ] **Step 2: Rewrite `src/server/handlers/sync.rs`**

Replace the **entire file** with:

```rust
use crate::server::sync::{self, device_flow, PendingDeviceFlow, SyncSession};
use crate::server::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Serialize)]
pub struct PendingLogin {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in_secs: u64,
}

#[derive(Serialize)]
pub struct SyncStatusResponse {
    pub logged_in: bool,
    pub email: Option<String>,
    pub syncing: bool,
    pub pending_login: Option<PendingLogin>,
}

pub async fn sync_status(State(state): State<Arc<AppState>>) -> Json<SyncStatusResponse> {
    let (logged_in, email, syncing) = state.sync_status().await;

    let pending_login = {
        let guard = state.pending_device_flow.lock().await;
        guard.as_ref().map(|p| PendingLogin {
            user_code: p.user_code.clone(),
            verification_uri: p.verification_uri.clone(),
            verification_uri_complete: p.verification_uri_complete.clone(),
            expires_in_secs: p
                .expires_at
                .saturating_duration_since(Instant::now())
                .as_secs(),
        })
    };

    Json(SyncStatusResponse {
        logged_in,
        email,
        syncing,
        pending_login,
    })
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
}

pub async fn sync_login(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Already logged in?
    if state.sync_session.lock().unwrap().is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Already logged in" })),
        ));
    }

    // Already a flow in progress?
    {
        let guard = state.pending_device_flow.lock().await;
        if guard.is_some() {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": "Login already in progress" })),
            ));
        }
    }

    let client = reqwest::Client::new();
    let name = device_flow::client_name(device_flow::server_host_label());
    let dc = device_flow::request_device_code(&client, &name)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": format!("cook.md unreachable: {e}") })),
            )
        })?;

    let cancel = CancellationToken::new();
    let expires_at = Instant::now() + Duration::from_secs(dc.expires_in);
    let interval = Duration::from_secs(dc.interval);

    let pending = PendingDeviceFlow {
        device_code: dc.device_code.clone(),
        user_code: dc.user_code.clone(),
        verification_uri: dc.verification_uri.clone(),
        verification_uri_complete: dc.verification_uri_complete.clone(),
        expires_at,
        interval,
        cancel: cancel.clone(),
    };

    *state.pending_device_flow.lock().await = Some(pending);

    let state_clone = state.clone();
    let device_code = dc.device_code.clone();
    tokio::spawn(async move {
        let result = device_flow::poll_for_token(
            &client,
            &device_code,
            interval,
            expires_at,
            cancel,
        )
        .await;

        // Clear pending flow regardless of outcome
        *state_clone.pending_device_flow.lock().await = None;

        match result {
            Ok(jwt) => {
                match SyncSession::from_jwt(jwt) {
                    Ok(session) => {
                        if let Err(e) = session.save(&state_clone.session_path) {
                            tracing::error!("Failed to save session: {e}");
                            return;
                        }
                        *state_clone.sync_session.lock().unwrap() = Some(session.clone());

                        match sync::sync_db_path() {
                            Ok(db_path) => match sync::start_sync(
                                &session,
                                state_clone.base_path.to_string(),
                                db_path,
                            ) {
                                Ok(handle) => {
                                    *state_clone.sync_handle.lock().await = Some(handle);
                                    tracing::info!("Sync started after login");
                                }
                                Err(e) => tracing::warn!("Failed to start sync after login: {e}"),
                            },
                            Err(e) => tracing::error!("Failed to resolve sync db path: {e}"),
                        }
                    }
                    Err(e) => tracing::error!("Failed to build SyncSession from JWT: {e}"),
                }
            }
            Err(device_flow::DeviceFlowError::Cancelled) => {
                tracing::info!("Login cancelled by user");
            }
            Err(e) => tracing::error!("Login failed: {e}"),
        }
    });

    Ok(Json(LoginResponse {
        user_code: dc.user_code,
        verification_uri: dc.verification_uri,
        verification_uri_complete: dc.verification_uri_complete,
        expires_in: dc.expires_in,
    }))
}

pub async fn sync_cancel_login(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let mut guard = state.pending_device_flow.lock().await;
    if let Some(p) = guard.take() {
        p.cancel.cancel();
        Json(serde_json::json!({ "cancelled": true }))
    } else {
        Json(serde_json::json!({ "cancelled": false }))
    }
}

pub async fn sync_logout(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Stop sync task
    if let Some(handle) = state.sync_handle.lock().await.take() {
        handle.stop().await;
    }

    // Clear session
    *state.sync_session.lock().unwrap() = None;
    if let Err(e) = SyncSession::delete(&state.session_path) {
        tracing::warn!("Failed to delete session file: {e}");
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
```

- [ ] **Step 3: Register the new route**

In `src/server/mod.rs`, find the existing `#[cfg(feature = "sync")]` route block (around lines 461-465):

```rust
#[cfg(feature = "sync")]
let router = router
    .route("/api/sync/status", get(handlers::sync_status))
    .route("/api/sync/login", post(handlers::sync_login))
    .route("/api/sync/logout", post(handlers::sync_logout));
```

Change it to:

```rust
#[cfg(feature = "sync")]
let router = router
    .route("/api/sync/status", get(handlers::sync_status))
    .route("/api/sync/login", post(handlers::sync_login))
    .route("/api/sync/cancel_login", post(handlers::sync_cancel_login))
    .route("/api/sync/logout", post(handlers::sync_logout));
```

- [ ] **Step 4: Export `sync_cancel_login` from handlers module**

Run: `grep -n 'pub use.*sync' src/server/handlers/mod.rs`

If the module re-exports via `pub use sync::*` you're done. Otherwise add `pub use sync::sync_cancel_login;` (or the appropriate explicit export) to match the existing pattern for `sync_login`, `sync_status`, `sync_logout`.

- [ ] **Step 5: Build**

Run: `cargo build --features sync`
Expected: clean build.

- [ ] **Step 6: Lint**

Run: `cargo fmt && cargo clippy --features sync -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Smoke test against a Phase-1 server**

In one terminal, run cook.md locally on port 3000 (Phase 1).

In another:
```
COOK_ENDPOINT=http://localhost:3000 cargo run --features sync -- server ./seed
```

In a browser at `http://localhost:9080/preferences`, click "Login to CookCloud". The button currently still triggers the *old* JS (templates/preferences.html is not yet updated). Verify in the network tab that `POST /api/sync/login` returns a body containing `user_code`, `verification_uri`, `verification_uri_complete`, `expires_in`. (The page won't render the new UI yet — that's Task 11.)

Cancel via:
```
curl -X POST http://localhost:9080/api/sync/cancel_login
```

Verify `GET /api/sync/status` no longer returns `pending_login`.

- [ ] **Step 8: Commit**

```bash
git add src/server/handlers/sync.rs src/server/mod.rs src/server/sync/mod.rs
git commit -m "feat(sync): replace browser-popup login with device-code flow"
```

---

### Task 11: Update `preferences.html` for device-code UI

**Files:**
- Modify: `templates/preferences.html`

- [ ] **Step 1: Read the current login section**

Run: `sed -n '40,80p' templates/preferences.html`
Run: `sed -n '155,225p' templates/preferences.html`

Note the existing markup for the Login button and the `syncLogin()` JS function. They will both be replaced.

- [ ] **Step 2: Replace the Login button markup**

Find the block containing `<button id="sync-login-btn">` (typically inside `{% if not sync_logged_in %}` around lines 60-72). Replace it with:

```html
<div id="sync-login-section">
  <button id="sync-login-btn"
          onclick="syncLogin()"
          class="px-4 py-2 rounded-md bg-cook-text text-white">
    Login to CookCloud
  </button>
  <p id="sync-login-message" class="text-sm text-cook-text/60 mt-2"></p>
</div>

<div id="sync-login-card" class="hidden mt-4 border border-cook-text/10 rounded-lg p-6 bg-white">
  <h3 class="text-lg font-semibold">Sign in to CookCloud</h3>
  <ol class="mt-3 space-y-2 text-sm">
    <li>1. Open <a id="sync-login-link" href="#" target="_blank" rel="noopener" class="underline">cook.md/device</a> in any browser.</li>
    <li>2. Enter this code:</li>
  </ol>
  <div class="mt-3 flex items-center gap-2">
    <code id="sync-login-code" class="text-2xl tracking-widest bg-neutral-100 px-4 py-2 rounded">----  ----</code>
    <button id="sync-login-copy" type="button" class="px-3 py-1 border rounded text-sm">Copy</button>
  </div>
  <p id="sync-login-expires" class="mt-3 text-sm text-cook-text/60"></p>
  <div class="mt-4 flex gap-2">
    <a id="sync-login-open" href="#" target="_blank" rel="noopener" class="px-4 py-2 rounded bg-cook-text text-white">Open cook.md/device</a>
    <button id="sync-login-cancel" type="button" class="px-4 py-2 rounded border">Cancel</button>
  </div>
</div>
```

- [ ] **Step 3: Replace the `syncLogin()` JS**

Find the `{% if sync_enabled %} ... async function syncLogin() ...` block (around lines 161-213). Replace from `async function syncLogin()` through the closing brace before `async function syncLogout()` with:

```html
let pollHandle = null;
let countdownHandle = null;

function renderPending(p) {
  document.getElementById('sync-login-section').classList.add('hidden');
  const card = document.getElementById('sync-login-card');
  card.classList.remove('hidden');
  document.getElementById('sync-login-code').textContent = p.user_code;
  document.getElementById('sync-login-link').href = p.verification_uri;
  document.getElementById('sync-login-link').textContent = p.verification_uri;
  document.getElementById('sync-login-open').href = p.verification_uri_complete;

  if (countdownHandle) clearInterval(countdownHandle);
  let remaining = p.expires_in_secs ?? p.expires_in;
  const expEl = document.getElementById('sync-login-expires');
  const tick = () => {
    if (remaining <= 0) {
      expEl.textContent = 'Code expired.';
      stopPolling();
      resetLoginUi('Code expired — try again.');
      return;
    }
    const m = Math.floor(remaining / 60);
    const s = String(remaining % 60).padStart(2, '0');
    expEl.textContent = `Expires in ${m}:${s}`;
    remaining--;
  };
  tick();
  countdownHandle = setInterval(tick, 1000);
}

function stopPolling() {
  if (pollHandle) { clearInterval(pollHandle); pollHandle = null; }
  if (countdownHandle) { clearInterval(countdownHandle); countdownHandle = null; }
}

function resetLoginUi(msg) {
  document.getElementById('sync-login-card').classList.add('hidden');
  document.getElementById('sync-login-section').classList.remove('hidden');
  document.getElementById('sync-login-message').textContent = msg || '';
}

async function syncLogin() {
  const btn = document.getElementById('sync-login-btn');
  const msg = document.getElementById('sync-login-message');
  try {
    btn.disabled = true;
    msg.textContent = 'Requesting code...';
    const resp = await fetch('{{ prefix }}/api/sync/login', { method: 'POST' });
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({}));
      btn.disabled = false;
      msg.textContent = err.error || 'Login failed.';
      return;
    }
    const body = await resp.json();
    renderPending({ ...body, expires_in_secs: body.expires_in });

    pollHandle = setInterval(async () => {
      const status = await fetch('{{ prefix }}/api/sync/status').then(r => r.json());
      if (status.logged_in) {
        stopPolling();
        window.location.reload();
      } else if (!status.pending_login) {
        stopPolling();
        resetLoginUi('Login was cancelled or expired.');
      }
    }, 2000);
  } catch (e) {
    btn.disabled = false;
    msg.textContent = 'Failed to start login: ' + e.message;
  }
}

document.addEventListener('DOMContentLoaded', () => {
  const copyBtn = document.getElementById('sync-login-copy');
  if (copyBtn) {
    copyBtn.addEventListener('click', async () => {
      const code = document.getElementById('sync-login-code').textContent;
      await navigator.clipboard.writeText(code.replace(/\s+/g, ''));
      copyBtn.textContent = 'Copied!';
      setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1500);
    });
  }
  const cancelBtn = document.getElementById('sync-login-cancel');
  if (cancelBtn) {
    cancelBtn.addEventListener('click', async () => {
      stopPolling();
      await fetch('{{ prefix }}/api/sync/cancel_login', { method: 'POST' });
      resetLoginUi('Login cancelled.');
    });
  }

  // Resume on page load if a flow is in progress server-side.
  fetch('{{ prefix }}/api/sync/status').then(r => r.json()).then(status => {
    if (!status.logged_in && status.pending_login) {
      renderPending(status.pending_login);
      pollHandle = setInterval(async () => {
        const s = await fetch('{{ prefix }}/api/sync/status').then(r => r.json());
        if (s.logged_in) { stopPolling(); window.location.reload(); }
        else if (!s.pending_login) { stopPolling(); resetLoginUi('Login was cancelled or expired.'); }
      }, 2000);
    }
  });
});
```

- [ ] **Step 4: Rebuild CSS**

Run: `make css` (or `npm run build-css`)
Expected: `static/css/output.css` regenerated with no errors.

- [ ] **Step 5: Manual UI smoke test**

Run cook.md locally on port 3000 (Phase 1 deployed). Then:

```
COOK_ENDPOINT=http://localhost:3000 cargo run --features sync -- server ./seed
```

In a browser at `http://localhost:9080/preferences`:
1. Click "Login to CookCloud". The login card appears with a code like `WDJB-MJHT` and an "Open cook.md/device" button.
2. Click "Open cook.md/device" → a new tab opens at `http://localhost:3000/device?user_code=WDJB-MJHT`. Sign in (email-code flow). The form pre-fills the code. Click "Continue", then "Approve".
3. Switch back to the CookCLI tab. Within ~2 s the page reloads and now shows the signed-in state.
4. Re-test the "Cancel" button. Re-test page-reload-mid-flow (refresh after clicking Login; the card should reappear with the same code).

If anything looks off visually, fix inline.

- [ ] **Step 6: Commit**

```bash
git add templates/preferences.html static/css/output.css
git commit -m "feat(sync): replace login button JS with device-code card"
```

---

### Task 12: `cook login` CLI command

**Files:**
- Create: `src/login.rs`
- Modify: `src/args.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Read the seed command pattern**

Run: `sed -n '1,40p' src/seed.rs`
Run: `grep -n 'Seed' src/args.rs src/main.rs`

Match the existing pattern.

- [ ] **Step 2: Create `src/login.rs`**

```rust
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use tokio_util::sync::CancellationToken;

use crate::server::sync::{self, device_flow, SyncSession};
use crate::Context;

#[derive(Debug, Parser)]
pub struct LoginArgs {}

pub fn run(_ctx: &Context, _args: LoginArgs) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(run_async())
}

async fn run_async() -> Result<()> {
    use std::io::{BufRead, Write};

    let session_path = crate::global_file_path("session.json")
        .unwrap_or_else(|_| std::path::PathBuf::from(".cook-session.json"));

    if SyncSession::load(&session_path).ok().flatten().is_some() {
        println!("Already logged in. Run `cook logout` first if you want to switch accounts.");
        return Ok(());
    }

    let client = reqwest::Client::new();
    let name = device_flow::client_name("cli");
    let dc = device_flow::request_device_code(&client, &name).await?;

    println!();
    println!("First open {} in any browser and enter this code:", dc.verification_uri);
    println!();
    println!("    {}", dc.user_code);
    println!();
    println!("(Press Enter to open it automatically, or Ctrl-C to abort.)");

    // Drain a line; ignore errors (e.g. piped stdin).
    let stdin = std::io::stdin();
    let _ = stdin.lock().lines().next();

    // Try to open the verification_uri_complete; failure is non-fatal.
    if let Err(e) = open::that(&dc.verification_uri_complete) {
        eprintln!("Couldn't open browser automatically: {e}");
        eprintln!("Please visit the URL above manually.");
    }

    print!("Waiting for authorization");
    std::io::stdout().flush().ok();

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel_for_signal.cancel();
    });

    let expires_at = Instant::now() + Duration::from_secs(dc.expires_in);
    let interval = Duration::from_secs(dc.interval);

    let dot_handle = {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        print!(".");
                        let _ = std::io::stdout().flush();
                    }
                }
            }
        })
    };

    let jwt = match device_flow::poll_for_token(&client, &dc.device_code, interval, expires_at, cancel.clone()).await {
        Ok(jwt) => jwt,
        Err(device_flow::DeviceFlowError::AccessDenied) => {
            cancel.cancel();
            dot_handle.abort();
            anyhow::bail!("Authorization denied.");
        }
        Err(device_flow::DeviceFlowError::Expired) => {
            cancel.cancel();
            dot_handle.abort();
            anyhow::bail!("Code expired — try `cook login` again.");
        }
        Err(device_flow::DeviceFlowError::Cancelled) => {
            dot_handle.abort();
            anyhow::bail!("Cancelled.");
        }
        Err(e) => {
            cancel.cancel();
            dot_handle.abort();
            anyhow::bail!("Login failed: {e}");
        }
    };

    cancel.cancel();
    dot_handle.abort();
    println!();

    let session = SyncSession::from_jwt(jwt)?;
    session.save(&session_path)?;

    let email = session.email().unwrap_or_else(|| "<unknown>".to_string());
    println!("✓ Logged in as {email}");
    println!();
    println!("Note: if `cook server` is running, restart it to pick up the new session.");

    let _ = sync::sync_db_path(); // touch / verify

    Ok(())
}
```

> **Note on `SyncSession::email()`:** if the existing API doesn't expose `email()` directly, use whatever public accessor returns the email (likely a `pub email: String` field or a method on the struct — check `src/server/sync/session.rs`). Adjust the call site to match.

- [ ] **Step 3: Add the variant to `args.rs`**

In `src/args.rs`, find the `Command` enum. Add:

```rust
/// Sign in to CookCloud.
#[cfg(feature = "sync")]
Login(crate::login::LoginArgs),
```

- [ ] **Step 4: Wire the dispatch in `main.rs`**

In `src/main.rs`, find the `match command { ... }` block. Add:

```rust
#[cfg(feature = "sync")]
Command::Login(args) => login::run(&ctx, args),
```

Also add the module declaration near the top:

```rust
#[cfg(feature = "sync")]
mod login;
```

- [ ] **Step 5: Build**

Run: `cargo build --features sync`
Expected: clean build.

- [ ] **Step 6: Lint**

Run: `cargo fmt && cargo clippy --features sync -- -D warnings`

- [ ] **Step 7: Manual smoke test**

With Phase 1 running locally:

```
COOK_ENDPOINT=http://localhost:3000 cargo run --features sync -- login
```

Press Enter when prompted. Browser opens to cook.md/device. Sign in, enter code, approve. Terminal shows "✓ Logged in as ...". Then:

```
ls -la ~/.config/cook/session.json   # or platform-equivalent
```

File exists with mode `-rw-------`.

Test the failure paths:
- Run `cook login`, then Ctrl-C → exits with "Cancelled."
- Run `cook login`, enter the code on web but click "Deny" → exits with "Authorization denied."

- [ ] **Step 8: Commit**

```bash
git add src/login.rs src/args.rs src/main.rs
git commit -m "feat(sync): add `cook login` CLI command"
```

---

### Task 13: `cook logout` CLI command

**Files:**
- Create: `src/logout.rs`
- Modify: `src/args.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/logout.rs`**

```rust
use anyhow::Result;
use clap::Parser;

use crate::server::sync::SyncSession;
use crate::Context;

#[derive(Debug, Parser)]
pub struct LogoutArgs {}

pub fn run(_ctx: &Context, _args: LogoutArgs) -> Result<()> {
    let session_path = crate::global_file_path("session.json")
        .unwrap_or_else(|_| std::path::PathBuf::from(".cook-session.json"));

    match SyncSession::load(&session_path) {
        Ok(Some(_)) => {
            SyncSession::delete(&session_path)?;
            println!("Logged out.");
        }
        _ => {
            println!("Not logged in.");
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Add the variant to `args.rs`**

```rust
/// Sign out of CookCloud.
#[cfg(feature = "sync")]
Logout(crate::logout::LogoutArgs),
```

- [ ] **Step 3: Wire dispatch in `main.rs`**

```rust
#[cfg(feature = "sync")]
mod logout;
```

```rust
#[cfg(feature = "sync")]
Command::Logout(args) => logout::run(&ctx, args),
```

- [ ] **Step 4: Build & lint**

Run: `cargo build --features sync && cargo fmt && cargo clippy --features sync -- -D warnings`

- [ ] **Step 5: Manual smoke test**

After running `cook login` in Task 12:

```
cargo run --features sync -- logout
ls -la ~/.config/cook/session.json   # should not exist
cargo run --features sync -- logout   # second invocation
```

Expected output, in order: `Logged out.`, then `Not logged in.`

- [ ] **Step 6: Commit**

```bash
git add src/logout.rs src/args.rs src/main.rs
git commit -m "feat(sync): add `cook logout` CLI command"
```

---

### Task 14: Final Phase-2 verification

- [ ] **Step 1: Full build + lint + format**

Run: `cargo build --features sync && cargo fmt --check && cargo clippy --features sync -- -D warnings && cargo test --features sync`
Expected: clean across the board. (`cargo test` may report `0 passed` — the codebase has no Rust tests yet. That's fine.)

- [ ] **Step 2: Build the non-sync variant**

Run: `cargo build --no-default-features --features self-update`
Expected: clean — confirms no accidental references to sync types from non-feature-gated code.

- [ ] **Step 3: Full manual test matrix**

For each row, complete the device-code flow end-to-end and confirm the session file is written.

| Environment | Command | Browser? |
|---|---|---|
| Native macOS | `cook login` | Default browser opens. |
| Native macOS, no browser auto-open | `cook login`, press Ctrl-C immediately after seeing the code | Cancels cleanly. |
| Inside Docker (`docker run -it cookcli login`) | `cook login` | Auto-open fails gracefully; user pastes URL into host browser. |
| SSH session, no display | `cook login` | Auto-open fails gracefully; user copies URL. |
| Web UI native | Browser → preferences → Login | New tab to cook.md; flow completes; preferences reloads. |
| Web UI in Docker (port-mapped 9080) | Browser → preferences → Login | Card shows code; user opens cook.md/device on host; flow completes. |

For each: also verify `cook logout` removes the session.

- [ ] **Step 4: Confirm the original issue is fixed**

Reproduce the issue from #290 if possible:

```yaml
services:
  cookcli:
    image: <local build tagged for test>
    ports: ["9080:9080"]
    volumes: ["./recipes:/recipes"]
```

Run `docker compose up`. Open `http://localhost:9080/preferences`. Click Login. Verify no `ERROR Failed to open browser` log line appears, and the device-code card renders.

- [ ] **Step 5: Squash-merge or PR**

Open the PR linking to issue #290 and the design spec. PR description should call out:
- Removed: server-side `open::that` for auth flow, local TCP listener / CSRF callback.
- Added: device-code flow shared by web UI and CLI; `cook login` / `cook logout`.
- Behaviour change: server-side login never spawns a host browser.

---

## Self-Review (already performed during plan-writing)

**1. Spec coverage:** every section of the spec maps to one or more tasks above:
- §"cook.md side" → Tasks 1-7
- §"CookCLI" device_flow module → Task 8
- AppState changes → Task 9 (folded into Task 10)
- handler rewrite → Task 10
- preferences.html → Task 11
- `cook login` → Task 12
- `cook logout` → Task 13
- "Files deleted" inventory verified inside Task 10 (rewrites the file wholesale).
- Security section: rate limits in Task 1; SHA-256 hashing in Task 2; CSRF tokens via Rails defaults in Task 5; single-use enforcement covered by Task 2 spec and Task 4 spec.
- Testing section: RSpec specs in Tasks 2-6; manual matrix in Task 14.

**2. Placeholder scan:** no "TBD", "TODO", "fill in details" present. The note about `SyncSession::email()` in Task 12 is an *adapt-to-existing-API* instruction with a clear fallback (use whatever the public accessor is), not a placeholder.

**3. Type consistency:** `PendingDeviceFlow`, `DeviceCodeResponse`, `LoginResponse`, `PendingLogin`, `SyncStatusResponse`, `DeviceFlowError` — names used consistently across Tasks 8, 9, 10, 11. `device_flow::client_name()` / `server_host_label()` / `request_device_code()` / `poll_for_token()` signatures match between definition (Task 8) and call sites (Tasks 10, 12). `DeviceFlow.create!` / `find_by_user_code` / `approve!` / `deny!` / `poll` signatures match between Task 2 spec, Task 2 impl, and call sites in Tasks 4-6.
