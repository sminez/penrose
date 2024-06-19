(function() {var type_impls = {
"penrose":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Conn%3CC%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#129-237\">source</a><a href=\"#impl-Conn%3CC%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;C&gt; <a class=\"struct\" href=\"penrose/x11rb/struct.Conn.html\" title=\"struct penrose::x11rb::Conn\">Conn</a>&lt;C&gt;<div class=\"where\">where\n    C: Connection,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.connection\" class=\"method\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#165-167\">source</a><h4 class=\"code-header\">pub fn <a href=\"penrose/x11rb/struct.Conn.html#tymethod.connection\" class=\"fn\">connection</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.reference.html\">&amp;C</a></h4></section></summary><div class=\"docblock\"><p>Get a handle to the underlying connection.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.create_window\" class=\"method\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#170-229\">source</a><h4 class=\"code-header\">pub fn <a href=\"penrose/x11rb/struct.Conn.html#tymethod.create_window\" class=\"fn\">create_window</a>(&amp;self, ty: <a class=\"enum\" href=\"penrose/x/enum.WinType.html\" title=\"enum penrose::x::WinType\">WinType</a>, r: <a class=\"struct\" href=\"penrose/pure/geometry/struct.Rect.html\" title=\"struct penrose::pure::geometry::Rect\">Rect</a>, managed: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.bool.html\">bool</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Create and map a new window to the screen with the specified <a href=\"penrose/x/enum.WinType.html\" title=\"enum penrose::x::WinType\">WinType</a>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.destroy_window\" class=\"method\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#232-236\">source</a><h4 class=\"code-header\">pub fn <a href=\"penrose/x11rb/struct.Conn.html#tymethod.destroy_window\" class=\"fn\">destroy_window</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Destroy the window identified by the given <code>Xid</code>.</p>\n</div></details></div></details>",0,"penrose::x11rb::RustConn"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Conn%3CRustConnection%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#104-112\">source</a><a href=\"#impl-Conn%3CRustConnection%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"struct\" href=\"penrose/x11rb/struct.Conn.html\" title=\"struct penrose::x11rb::Conn\">Conn</a>&lt;RustConnection&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#107-111\">source</a><h4 class=\"code-header\">pub fn <a href=\"penrose/x11rb/struct.Conn.html#tymethod.new\" class=\"fn\">new</a>() -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;Self&gt;</h4></section></summary><div class=\"docblock\"><p>Construct an X11rbConnection  backed by the <a href=\"penrose/x11rb/index.html\" title=\"mod penrose::x11rb\">x11rb</a> backend using\n[x11rb::rust_connection::RustConnection].</p>\n</div></details></div></details>",0,"penrose::x11rb::RustConn"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-Conn%3CC%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#94\">source</a><a href=\"#impl-Debug-for-Conn%3CC%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;C: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + Connection&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"penrose/x11rb/struct.Conn.html\" title=\"struct penrose::x11rb::Conn\">Conn</a>&lt;C&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#94\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","penrose::x11rb::RustConn"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-XConn-for-Conn%3CC%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#239-711\">source</a><a href=\"#impl-XConn-for-Conn%3CC%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;C&gt; <a class=\"trait\" href=\"penrose/x/trait.XConn.html\" title=\"trait penrose::x::XConn\">XConn</a> for <a class=\"struct\" href=\"penrose/x11rb/struct.Conn.html\" title=\"struct penrose::x11rb::Conn\">Conn</a>&lt;C&gt;<div class=\"where\">where\n    C: Connection,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.root\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#243-245\">source</a><a href=\"#method.root\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.root\" class=\"fn\">root</a>(&amp;self) -&gt; <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a></h4></section></summary><div class='docblock'>The ID of the window manager root window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.screen_details\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#247-276\">source</a><a href=\"#method.screen_details\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.screen_details\" class=\"fn\">screen_details</a>(&amp;self) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"penrose/pure/geometry/struct.Rect.html\" title=\"struct penrose::pure::geometry::Rect\">Rect</a>&gt;&gt;</h4></section></summary><div class='docblock'>Ask the X server for the dimensions of each currently available screen.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.cursor_position\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#278-282\">source</a><a href=\"#method.cursor_position\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.cursor_position\" class=\"fn\">cursor_position</a>(&amp;self) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"penrose/pure/geometry/struct.Point.html\" title=\"struct penrose::pure::geometry::Point\">Point</a>&gt;</h4></section></summary><div class='docblock'>Ask the X server for the current (x, y) coordinate of the mouse cursor.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.grab\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#284-332\">source</a><a href=\"#method.grab\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.grab\" class=\"fn\">grab</a>(&amp;self, key_codes: &amp;[<a class=\"struct\" href=\"penrose/core/bindings/struct.KeyCode.html\" title=\"struct penrose::core::bindings::KeyCode\">KeyCode</a>], mouse_states: &amp;[<a class=\"struct\" href=\"penrose/core/bindings/struct.MouseState.html\" title=\"struct penrose::core::bindings::MouseState\">MouseState</a>]) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Grab the specified key and mouse states, intercepting them for processing within\nthe window manager itself.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.next_event\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#334-341\">source</a><a href=\"#method.next_event\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.next_event\" class=\"fn\">next_event</a>(&amp;self) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"enum\" href=\"penrose/x/event/enum.XEvent.html\" title=\"enum penrose::x::event::XEvent\">XEvent</a>&gt;</h4></section></summary><div class='docblock'>Block and wait for the next event from the X server so it can be processed.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.flush\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#343-345\">source</a><a href=\"#method.flush\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.flush\" class=\"fn\">flush</a>(&amp;self)</h4></section></summary><div class='docblock'>Flush any pending events to the X server.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.intern_atom\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#347-354\">source</a><a href=\"#method.intern_atom\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.intern_atom\" class=\"fn\">intern_atom</a>(&amp;self, atom: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.str.html\">str</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>&gt;</h4></section></summary><div class='docblock'>Look up the <a href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a> of a given <a href=\"penrose/x/atom/enum.Atom.html\" title=\"enum penrose::x::atom::Atom\">Atom</a> name. If it is not currently interned, intern it.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.atom_name\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#356-367\">source</a><a href=\"#method.atom_name\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.atom_name\" class=\"fn\">atom_name</a>(&amp;self, xid: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>&gt;</h4></section></summary><div class='docblock'>Look up the string name of a given <a href=\"penrose/x/atom/enum.Atom.html\" title=\"enum penrose::x::atom::Atom\">Atom</a> by its <a href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.client_geometry\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#369-378\">source</a><a href=\"#method.client_geometry\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.client_geometry\" class=\"fn\">client_geometry</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"penrose/pure/geometry/struct.Rect.html\" title=\"struct penrose::pure::geometry::Rect\">Rect</a>&gt;</h4></section></summary><div class='docblock'>Look up the current dimensions and position of a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.existing_clients\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#380-385\">source</a><a href=\"#method.existing_clients\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.existing_clients\" class=\"fn\">existing_clients</a>(&amp;self) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>&gt;&gt;</h4></section></summary><div class='docblock'>Ask the X server for the IDs of all currently known client windows</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.map\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#387-391\">source</a><a href=\"#method.map\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.map\" class=\"fn\">map</a>(&amp;self, client: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Map the given client window to the screen with its current geometry, making it visible.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.unmap\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#393-397\">source</a><a href=\"#method.unmap\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.unmap\" class=\"fn\">unmap</a>(&amp;self, client: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Unmap the given client window from the screen, hiding it.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.kill\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#399-413\">source</a><a href=\"#method.kill\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.kill\" class=\"fn\">kill</a>(&amp;self, client: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Kill the given client window, closing it.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.focus\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#415-420\">source</a><a href=\"#method.focus\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.focus\" class=\"fn\">focus</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Set X input focus to be held by the given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_prop\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#422-523\">source</a><a href=\"#method.get_prop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.get_prop\" class=\"fn\">get_prop</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>, prop_name: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.str.html\">str</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"enum\" href=\"penrose/x/property/enum.Prop.html\" title=\"enum penrose::x::property::Prop\">Prop</a>&gt;&gt;</h4></section></summary><div class='docblock'>Look up a specific property on a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.list_props\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#525-533\">source</a><a href=\"#method.list_props\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.list_props\" class=\"fn\">list_props</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>&gt;&gt;</h4></section></summary><div class='docblock'>List the known property names set for a given client.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.delete_prop\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#535-540\">source</a><a href=\"#method.delete_prop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.delete_prop\" class=\"fn\">delete_prop</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>, prop_name: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.str.html\">str</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Delete a property for a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_window_attributes\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#542-564\">source</a><a href=\"#method.get_window_attributes\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.get_window_attributes\" class=\"fn\">get_window_attributes</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"struct\" href=\"penrose/x/property/struct.WindowAttributes.html\" title=\"struct penrose::x::property::WindowAttributes\">WindowAttributes</a>&gt;</h4></section></summary><div class='docblock'>Request the <a href=\"penrose/x/property/struct.WindowAttributes.html\" title=\"struct penrose::x::property::WindowAttributes\">WindowAttributes</a> for a given client window from the X server.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_wm_state\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#566-577\">source</a><a href=\"#method.get_wm_state\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.get_wm_state\" class=\"fn\">get_wm_state</a>(&amp;self, client: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"enum\" href=\"penrose/x/property/enum.WmState.html\" title=\"enum penrose::x::property::WmState\">WmState</a>&gt;&gt;</h4></section></summary><div class='docblock'>Get the current <a href=\"penrose/x/property/enum.WmState.html\" title=\"enum penrose::x::property::WmState\">WmState</a> for a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.set_wm_state\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#579-591\">source</a><a href=\"#method.set_wm_state\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.set_wm_state\" class=\"fn\">set_wm_state</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>, wm_state: <a class=\"enum\" href=\"penrose/x/property/enum.WmState.html\" title=\"enum penrose::x::property::WmState\">WmState</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Set the current <a href=\"penrose/x/property/enum.WmState.html\" title=\"enum penrose::x::property::WmState\">WmState</a> for a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.set_prop\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#593-631\">source</a><a href=\"#method.set_prop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.set_prop\" class=\"fn\">set_prop</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>, name: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.str.html\">str</a>, val: <a class=\"enum\" href=\"penrose/x/property/enum.Prop.html\" title=\"enum penrose::x::property::Prop\">Prop</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Set a specific property on a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.set_client_attributes\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#633-659\">source</a><a href=\"#method.set_client_attributes\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.set_client_attributes\" class=\"fn\">set_client_attributes</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>, attrs: &amp;[<a class=\"enum\" href=\"penrose/x/enum.ClientAttr.html\" title=\"enum penrose::x::ClientAttr\">ClientAttr</a>]) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Set one or more <a href=\"penrose/x/enum.ClientAttr.html\" title=\"enum penrose::x::ClientAttr\">ClientAttr</a> for a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.set_client_config\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#661-678\">source</a><a href=\"#method.set_client_config\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.set_client_config\" class=\"fn\">set_client_config</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>, data: &amp;[<a class=\"enum\" href=\"penrose/x/enum.ClientConfig.html\" title=\"enum penrose::x::ClientConfig\">ClientConfig</a>]) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Set the <a href=\"penrose/x/enum.ClientConfig.html\" title=\"enum penrose::x::ClientConfig\">ClientConfig</a> for a given client window.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.send_client_message\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#680-704\">source</a><a href=\"#method.send_client_message\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.send_client_message\" class=\"fn\">send_client_message</a>(&amp;self, msg: <a class=\"struct\" href=\"penrose/x/event/struct.ClientMessage.html\" title=\"struct penrose::x::event::ClientMessage\">ClientMessage</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Send a <a href=\"penrose/x/event/struct.ClientMessage.html\" title=\"struct penrose::x::event::ClientMessage\">ClientMessage</a> to a given client.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.warp_pointer\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/penrose/x11rb/mod.rs.html#706-710\">source</a><a href=\"#method.warp_pointer\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"penrose/x/trait.XConn.html#tymethod.warp_pointer\" class=\"fn\">warp_pointer</a>(&amp;self, id: <a class=\"struct\" href=\"penrose/struct.Xid.html\" title=\"struct penrose::Xid\">Xid</a>, x: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.i16.html\">i16</a>, y: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.i16.html\">i16</a>) -&gt; <a class=\"type\" href=\"penrose/type.Result.html\" title=\"type penrose::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Reposition the mouse cursor to the given (x, y) coordinates within the specified window.\nThis method should not be called directly: use <code>warp_pointer_to_window</code> or <code>warp_pointer_to_screen</code>\ninstead.</div></details></div></details>","XConn","penrose::x11rb::RustConn"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()