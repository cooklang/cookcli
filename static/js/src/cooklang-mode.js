import { StreamLanguage } from "@codemirror/language";

// Cooklang syntax highlighting mode for CodeMirror 6
// Ported from cooklang-obsidian/src/mode/cook/cook.ts
export const cooklang = StreamLanguage.define({
  name: "cooklang",

  startState() {
    return {
      formatting: false,
      nextMultiline: false,
      inMultiline: false,
      afterSection: false,
      position: null,
      inFrontmatter: false,
      inMetadata: false,
      inNote: false,
      inComment: false,
      afterAmount: false  // Track if we just closed a {} to look for (prep)
    };
  },

  token(stream, state) {
    const sol = stream.sol() || state.afterSection;
    const eol = stream.eol();

    state.afterSection = false;

    if (sol) {
      // Reset single-line states at start of new line
      state.inNote = false;
      state.inMetadata = false;

      if (state.nextMultiline) {
        state.inMultiline = true;
        state.nextMultiline = false;
      } else {
        state.position = null;
      }
    }

    if (eol && !state.nextMultiline) {
      state.inMultiline = false;
      state.position = null;
    }

    if (sol) {
      while (stream.eatSpace()) {}
    }

    // Frontmatter delimiters (---)
    if (sol && stream.match(/^---\s*$/)) {
      state.inFrontmatter = !state.inFrontmatter;
      return "meta";
    }

    // Inside frontmatter
    if (state.inFrontmatter) {
      stream.skipToEnd();
      return "meta";
    }

    // Sections (= Section Name, == Section ==, etc.)
    if (sol && stream.match(/^=+/)) {
      stream.skipToEnd();
      return "heading";
    }

    // Line comments (-- comment)
    if (sol && stream.match(/^--/)) {
      stream.skipToEnd();
      return "comment";
    }

    // Block comments ([- comment -])
    if (stream.match(/^\[-/)) {
      state.inComment = true;
      return "comment";
    }

    if (state.inComment) {
      if (stream.match(/-]/)) {
        state.inComment = false;
        return "comment";
      }
      stream.skipToEnd();
      return "comment";
    }

    // Metadata (>> key: value)
    if (sol && stream.match(/^>>/)) {
      state.inMetadata = true;
      state.position = "metadata-key";
      return "meta";
    }

    if (state.inMetadata) {
      if (state.position === "metadata-key") {
        if (stream.match(/^[^:]+:/)) {
          state.position = "metadata-value";
          return "meta";
        }
        stream.skipToEnd();
        return "meta";
      } else if (state.position === "metadata-value") {
        stream.skipToEnd();
        return "meta";
      }
    }

    // Notes (lines starting with >)
    if (sol && stream.match(/^>/)) {
      state.inNote = true;
      return "comment";
    }

    if (state.inNote) {
      stream.skipToEnd();
      return "comment";
    }

    // Shorthand preparations (prep) after ingredient amounts like @onion{1}(chopped)
    if (state.afterAmount && stream.match(/^\([^)]*\)/)) {
      state.afterAmount = false;
      return "string";  // Use string style for prep instructions
    }

    // Reset afterAmount if we see anything else
    if (state.afterAmount && !stream.match(/^\s*(?=\()/, false)) {
      state.afterAmount = false;
    }

    // Ingredients (@ingredient{amount})
    if (stream.match(/^@([^@#~]+?(?={))/)) {
      return "variableName";
    } else if (stream.match(/^@(.+?\b)/)) {
      return "variableName";
    }

    // Cookware (#cookware{amount})
    if (stream.match(/^#([^@#~]+?(?={))/)) {
      return "keyword";
    } else if (stream.match(/^#(.+?\b)/)) {
      return "keyword";
    }

    // Timers (~timer{amount})
    if (stream.match(/^~([^@#~]+?(?={))/)) {
      return "number";
    } else if (stream.match(/^~(.+?\b)/)) {
      return "number";
    }

    // Amounts in curly braces
    const ch = stream.next();
    if (!ch) return null;

    if (ch === '{') {
      if (state.position !== "timer") state.position = "measurement";
      return null;
    }

    if (ch === '}') {
      state.position = null;
      state.afterAmount = true;  // Look for (prep) after closing brace
      return null;
    }

    if (ch === '%' && (state.position === "measurement" || state.position === "timer")) {
      state.position = "unit";
      return null;
    }

    return state.position;
  }
});
