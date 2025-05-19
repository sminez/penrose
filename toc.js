// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">User Guide</li><li class="chapter-item expanded "><a href="getting-started.html"><strong aria-hidden="true">1.</strong> Getting Started</a></li><li class="chapter-item expanded "><a href="builtin/index.html"><strong aria-hidden="true">2.</strong> Built In Functionality</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="builtin/layouts.html"><strong aria-hidden="true">2.1.</strong> Layouts</a></li><li class="chapter-item expanded "><a href="builtin/actions.html"><strong aria-hidden="true">2.2.</strong> Actions</a></li><li class="chapter-item expanded "><a href="builtin/ui.html"><strong aria-hidden="true">2.3.</strong> UI</a></li></ol></li><li class="chapter-item expanded "><a href="extensions/index.html"><strong aria-hidden="true">3.</strong> Extensions</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="extensions/hooks.html"><strong aria-hidden="true">3.1.</strong> Hooks</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="extensions/startup-hooks.html"><strong aria-hidden="true">3.1.1.</strong> Startup Hooks</a></li><li class="chapter-item expanded "><a href="extensions/event-hooks.html"><strong aria-hidden="true">3.1.2.</strong> Event Hooks</a></li><li class="chapter-item expanded "><a href="extensions/manage-hooks.html"><strong aria-hidden="true">3.1.3.</strong> Manage Hooks</a></li><li class="chapter-item expanded "><a href="extensions/refresh-hooks.html"><strong aria-hidden="true">3.1.4.</strong> Refresh Hooks</a></li></ol></li><li class="chapter-item expanded "><a href="extensions/ewmh.html"><strong aria-hidden="true">3.2.</strong> EWMH</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Reference Guide</li><li class="chapter-item expanded "><a href="overview/index.html"><strong aria-hidden="true">4.</strong> Overview of Concepts</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="overview/pure-vs-x.html"><strong aria-hidden="true">4.1.</strong> Pure Code vs X Code</a></li><li class="chapter-item expanded "><a href="overview/data-structures.html"><strong aria-hidden="true">4.2.</strong> Data Structures</a></li></ol></li><li class="chapter-item expanded "><a href="building/index.html"><strong aria-hidden="true">5.</strong> Building on top of penrose</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="building/actions.html"><strong aria-hidden="true">5.1.</strong> Actions</a></li><li class="chapter-item expanded "><a href="building/layouts.html"><strong aria-hidden="true">5.2.</strong> Layouts</a></li><li class="chapter-item expanded "><div><strong aria-hidden="true">5.3.</strong> Hooks</div></li><li class="chapter-item expanded "><div><strong aria-hidden="true">5.4.</strong> State Extensions</div></li><li class="chapter-item expanded "><div><strong aria-hidden="true">5.5.</strong> Example: Named Scratchpads</div></li></ol></li><li class="chapter-item expanded "><a href="faq.html">FAQs</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
