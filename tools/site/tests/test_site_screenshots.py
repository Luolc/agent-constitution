"""Tests for site_screenshots.

The pages, schemes and widths give one shot each, named by the path,
the scheme and the width; a path maps to the file a static server
sends; the Chrome command carries the scheme's switch, the width and
--no-proxy-server.
"""

from pathlib import Path
import unittest

from constitution_site import site_screenshots
from constitution_site.site_screenshots import Shot


class ShotsTest(unittest.TestCase):
  def test_every_scheme_and_width_of_every_page(self):
    self.assertEqual(
      site_screenshots.shots(["/", "/zh/rules/79/"]),
      [
        Shot("/", "light", "desktop"),
        Shot("/", "light", "mobile"),
        Shot("/", "dark", "desktop"),
        Shot("/", "dark", "mobile"),
        Shot("/zh/rules/79/", "light", "desktop"),
        Shot("/zh/rules/79/", "light", "mobile"),
        Shot("/zh/rules/79/", "dark", "desktop"),
        Shot("/zh/rules/79/", "dark", "mobile"),
      ],
    )

  def test_names(self):
    self.assertEqual(
      Shot("/", "light", "desktop").name, "index.light.desktop.png"
    )
    self.assertEqual(
      Shot("/zh/rules/79/", "dark", "mobile").name,
      "zh-rules-79.dark.mobile.png",
    )

  def test_default_pages_have_distinct_names(self):
    names = [
      shot.name
      for shot in site_screenshots.shots(site_screenshots.DEFAULT_PAGES)
    ]
    self.assertEqual(len(names), len(set(names)))


class PageFileTest(unittest.TestCase):
  def test_directory_path_is_its_index(self):
    site = Path("site")
    self.assertEqual(site_screenshots.page_file(site, "/"), site / "index.html")
    self.assertEqual(
      site_screenshots.page_file(site, "/zh/rules/"),
      site / "zh/rules/index.html",
    )
    self.assertEqual(
      site_screenshots.page_file(site, "/404.html"), site / "404.html"
    )


class ChromeCommandTest(unittest.TestCase):
  def command(self, scheme: str, width: str) -> list[str]:
    return site_screenshots.chrome_command(
      "chrome",
      Path("profile"),
      Shot("/rules/", scheme, width),
      "http://127.0.0.1:1/rules/",
      Path("out.png"),
    )

  def test_dark_mobile(self):
    command = self.command("dark", "mobile")
    self.assertIn("--force-dark-mode", command)
    self.assertIn("--window-size=390,844", command)
    self.assertIn("--no-proxy-server", command)
    self.assertEqual(
      command[-2:], ["--screenshot=out.png", "http://127.0.0.1:1/rules/"]
    )

  def test_light_desktop(self):
    command = self.command("light", "desktop")
    self.assertNotIn("--force-dark-mode", command)
    self.assertIn("--window-size=1280,900", command)


if __name__ == "__main__":
  unittest.main()
