mod common;
pub mod menus;
pub mod pantry;
pub mod recipes;
pub mod shopping_list;
pub mod stats;
#[cfg(feature = "sync")]
pub mod sync;

pub use menus::{get_menu, list_menus};
pub use pantry::{
    add_item as add_pantry_item, get_depleted, get_expiring, get_pantry,
    remove_item as remove_pantry_item, update_item as update_pantry_item,
};
pub use recipes::{all_recipes, recipe, recipe_delete, recipe_raw, recipe_save, reload, search};
pub use shopping_list::{
    add_to_shopping_list, clear_shopping_list, get_shopping_list_items, remove_from_shopping_list,
    shopping_list,
};
pub use stats::stats;
#[cfg(feature = "sync")]
pub use sync::{sync_login, sync_logout, sync_status};
