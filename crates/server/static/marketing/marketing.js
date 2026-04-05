(function () {
  if (!document.body.classList.contains("marketing-page")) {
    return;
  }

  var prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  initMenuToggle();
  initRevealObserver(prefersReducedMotion);
  initCopyButtons();
  initTrackedLinks(".marketing-nav a[href^='#']");
  initTrackedLinks(".marketing-doc-links a[href^='#']");

  function initMenuToggle() {
    var toggle = document.querySelector("[data-marketing-menu-toggle]");
    if (!toggle) {
      return;
    }

    var closeMenu = function () {
      document.body.classList.remove("marketing-nav-open");
      toggle.setAttribute("aria-expanded", "false");
    };

    toggle.addEventListener("click", function () {
      var willOpen = !document.body.classList.contains("marketing-nav-open");
      document.body.classList.toggle("marketing-nav-open", willOpen);
      toggle.setAttribute("aria-expanded", willOpen ? "true" : "false");
    });

    window.addEventListener("resize", function () {
      if (window.innerWidth > 1024) {
        closeMenu();
      }
    });

    Array.prototype.forEach.call(document.querySelectorAll(".marketing-nav a"), function (link) {
      link.addEventListener("click", closeMenu);
    });
  }

  function initRevealObserver(reducedMotion) {
    var items = document.querySelectorAll("[data-reveal]");
    if (!items.length) {
      return;
    }

    if (reducedMotion || !("IntersectionObserver" in window)) {
      Array.prototype.forEach.call(items, function (item) {
        item.classList.add("is-visible");
      });
      return;
    }

    var observer = new IntersectionObserver(
      function (entries) {
        Array.prototype.forEach.call(entries, function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.18, rootMargin: "0px 0px -8% 0px" }
    );

    Array.prototype.forEach.call(items, function (item) {
      observer.observe(item);
    });
  }

  function initCopyButtons() {
    if (!navigator.clipboard) {
      return;
    }

    Array.prototype.forEach.call(document.querySelectorAll("pre[data-copyable]"), function (block) {
      var button = document.createElement("button");
      var resetTimer = 0;
      var sourceText = (block.querySelector("code") || block).innerText.trim();

      button.className = "marketing-copy-button";
      button.type = "button";
      button.textContent = "Copy";

      button.addEventListener("click", function () {
        navigator.clipboard.writeText(sourceText).then(
          function () {
            button.textContent = "Copied";
            window.clearTimeout(resetTimer);
            resetTimer = window.setTimeout(function () {
              button.textContent = "Copy";
            }, 1600);
          },
          function () {
            button.textContent = "Failed";
            window.clearTimeout(resetTimer);
            resetTimer = window.setTimeout(function () {
              button.textContent = "Copy";
            }, 1600);
          }
        );
      });

      block.classList.add("marketing-copy-ready");
      block.appendChild(button);
    });
  }

  function initTrackedLinks(selector) {
    var links = Array.prototype.slice.call(document.querySelectorAll(selector));
    if (!links.length || !("IntersectionObserver" in window)) {
      return;
    }

    var sections = links
      .map(function (link) {
        var target = link.getAttribute("href");
        return target && target.charAt(0) === "#" ? document.querySelector(target) : null;
      })
      .filter(Boolean);

    if (!sections.length) {
      return;
    }

    var setActive = function (id) {
      Array.prototype.forEach.call(links, function (link) {
        var active = link.getAttribute("href") === "#" + id;
        link.classList.toggle("is-active", active);
        if (active) {
          link.setAttribute("aria-current", "true");
        } else {
          link.removeAttribute("aria-current");
        }
      });
    };

    var observer = new IntersectionObserver(
      function (entries) {
        var visibleEntry = entries
          .filter(function (entry) {
            return entry.isIntersecting;
          })
          .sort(function (left, right) {
            return right.intersectionRatio - left.intersectionRatio;
          })[0];

        if (visibleEntry && visibleEntry.target.id) {
          setActive(visibleEntry.target.id);
        }
      },
      { threshold: [0.2, 0.45, 0.7], rootMargin: "-35% 0px -45% 0px" }
    );

    Array.prototype.forEach.call(sections, function (section) {
      observer.observe(section);
    });

    if (window.location.hash) {
      setActive(window.location.hash.slice(1));
    } else if (sections[0].id) {
      setActive(sections[0].id);
    }
  }
})();
