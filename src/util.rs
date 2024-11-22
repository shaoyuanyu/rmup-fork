/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use unicode_width::UnicodeWidthStr;

/// Make the given string exactly the width specified, truncating with elipses
/// or adding padding as necessary.
pub fn to_width(s: &str, width: usize, right_align: bool) -> String {
    let mut s_width = UnicodeWidthStr::width(s);
    let mut s = s.to_owned();
    let elipses = s_width >= width - 1;

    while elipses && s_width >= width - 4 {
        s.pop();
        s_width = UnicodeWidthStr::width(s.as_str());
    }

    if elipses {
        s.push_str("...");
        s_width = UnicodeWidthStr::width(s.as_str());
    }

    for _ in 0..(width - s_width) {
        if right_align {
            s.insert(0, ' ');
        } else {
            s.push(' ');
        }
    }

    s
}
