//! .gitignore templates and licence texts offered when creating a repository.

pub const GITIGNORES: &[(&str, &str)] = &[
    ("Rust", "/target\n**/*.rs.bk\n*.pdb\n"),
    ("Node", "node_modules/\nnpm-debug.log*\nyarn-error.log\n.env\ndist/\nbuild/\ncoverage/\n"),
    ("Python", "__pycache__/\n*.py[cod]\n*.egg-info/\n.venv/\nvenv/\n.env\ndist/\nbuild/\n.pytest_cache/\n"),
    ("Java", "*.class\n*.jar\n*.war\ntarget/\nbuild/\n.gradle/\n.idea/\n*.iml\n"),
    ("VisualStudio", "bin/\nobj/\n.vs/\n*.user\n*.suo\npackages/\n*.nupkg\n"),
    ("Go", "*.exe\n*.test\n*.out\nvendor/\n"),
    ("Unity", "/[Ll]ibrary/\n/[Tt]emp/\n/[Oo]bj/\n/[Bb]uild/\n/[Bb]uilds/\n/[Ll]ogs/\n/[Uu]ser[Ss]ettings/\n*.csproj\n*.sln\n"),
    ("C++", "*.o\n*.obj\n*.exe\n*.dll\n*.so\n*.dylib\n*.a\n*.lib\nbuild/\n"),
    ("macOS", ".DS_Store\n.AppleDouble\n.LSOverride\n._*\n"),
    ("Windows", "Thumbs.db\nehthumbs.db\nDesktop.ini\n$RECYCLE.BIN/\n"),
];

const MIT: &str = "MIT License

Copyright (c) {year} {name}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
";

const ISC: &str = "ISC License

Copyright (c) {year} {name}

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED \"AS IS\" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
";

const UNLICENSE: &str = "This is free and unencumbered software released into the public domain.

Anyone is free to copy, modify, publish, use, compile, sell, or
distribute this software, either in source code form or as a compiled
binary, for any purpose, commercial or non-commercial, and by any
means.

For more information, please refer to <https://unlicense.org>
";

const APACHE: &str = "Copyright {year} {name}

Licensed under the Apache License, Version 2.0 (the \"License\");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an \"AS IS\" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
";

const GPL3: &str = "Copyright (C) {year} {name}

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
";

pub const LICENSES: &[(&str, &str)] = &[
    ("MIT", MIT),
    ("Apache-2.0", APACHE),
    ("GPL-3.0", GPL3),
    ("ISC", ISC),
    ("Unlicense", UNLICENSE),
];

pub fn license_text(name: &str, owner: &str) -> String {
    let year = chrono::Local::now().format("%Y").to_string();
    LICENSES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, t)| t.replace("{year}", &year).replace("{name}", owner))
        .unwrap_or_default()
}

pub fn gitignore(name: &str) -> String {
    GITIGNORES.iter().find(|(n, _)| *n == name).map(|(_, t)| t.to_string()).unwrap_or_default()
}
