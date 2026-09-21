/*
 * Copyright (c) Axe. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 */
fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("src/resssources/favicon.ico");
        res.compile().unwrap();
    }
}