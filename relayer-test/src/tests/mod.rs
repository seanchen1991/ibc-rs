/*!
   All test cases are placed within this module.

   We expose the modules as public so that cargo doc
   will pick up the definition by default.
*/

pub mod memo;
pub mod transfer;

#[cfg(any(doc, feature = "example"))]
pub mod example;
