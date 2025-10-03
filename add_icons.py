#!/usr/bin/env python3
"""
Script to add missing icons from assets/icons-ext/ to the icon system.
This script will:
1. Find all SVG files in assets/icons-ext/
2. Convert filenames to PascalCase enum variants
3. Update crates/ui/src/icon.rs with new icons
4. Move icons from icons-ext to icons directory
"""

import os
import re
import shutil
from pathlib import Path

def kebab_to_pascal(kebab_str):
    """Convert kebab-case to PascalCase, handling numbers and special cases."""
    # Handle special cases for numbers at the start
    if kebab_str[0].isdigit():
        # e.g., "3d" -> "ThreeD", "4k" -> "FourK"
        number_words = {
            '0': 'Zero', '1': 'One', '2': 'Two', '3': 'Three', '4': 'Four',
            '5': 'Five', '6': 'Six', '7': 'Seven', '8': 'Eight', '9': 'Nine'
        }
        parts = kebab_str.split('-')
        if parts[0] in number_words:
            parts[0] = number_words[parts[0]]

    # Split by hyphen and capitalize each part
    parts = kebab_str.split('-')
    pascal = ''.join(word.capitalize() for word in parts)

    # Handle special cases
    replacements = {
        'Svg': 'SVG',
        'Png': 'PNG',
        'Jpg': 'JPG',
        'Jpeg': 'JPEG',
        'Gif': 'GIF',
        'Html': 'HTML',
        'Css': 'CSS',
        'Json': 'JSON',
        'Xml': 'XML',
        'Api': 'API',
        'Url': 'URL',
        'Uri': 'URI',
        'Id': 'ID',
        'Ui': 'UI',
        'Ux': 'UX',
        'Usb': 'USB',
        'Hdmi': 'HDMI',
        'Wifi': 'WiFi',
        'Gps': 'GPS',
        'Cpu': 'CPU',
        'Ram': 'RAM',
        'Ssd': 'SSD',
        'Hdd': 'HDD',
        'Ai': 'AI',
        'Ar': 'AR',
        'Vr': 'VR',
        'Qr': 'QR',
        'Nfc': 'NFC',
        'Ev': 'EV',
        'Tv': 'TV',
        'Hd': 'HD',
        'Fps': 'FPS',
        'Dns': 'DNS',
        'Vpn': 'VPN',
        'Rss': 'RSS',
        'Npm': 'NPM',
        'Github': 'GitHub',
        'Gitlab': 'GitLab',
        'Iphone': 'iPhone',
        'Ipad': 'iPad',
        'Imac': 'iMac',
        'Macos': 'MacOS',
        'Ios': 'iOS',
        'Ipv': 'IPv',
        'Okrs': 'OKRs',
        'Gif': 'GIF',
        'Tiff': 'TIFF',
        'Jpeg': 'JPEG',
        'Webp': 'WEBP',
        'Mpeg': 'MPEG',
        'Avi': 'AVI',
        'Tif': 'TIF',
        'Raw': 'RAW',
        'Css3': 'CSS3',
        'Html5': 'HTML5',
        'Db': 'DB',
        'Api': 'API',
        '3d': 'ThreeD',
        '3D': 'ThreeD',
        '4k': 'FourK',
        '4K': 'FourK',
        '2x2': 'TwoByTwo',
        '15': 'Fifteen',
        '45deg': 'FortyFiveDeg',
        '360': 'ThreeSixty',
        '2021': 'TwentyTwentyOne',
        '1st': 'First',
        '25': 'TwentyFive',
        '50': 'Fifty',
        '75': 'SeventyFive',
    }

    for old, new in replacements.items():
        pascal = pascal.replace(old, new)

    return pascal

def get_existing_icons(icon_rs_path):
    """Parse icon.rs to get existing icon names."""
    with open(icon_rs_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Find all enum variants
    enum_match = re.search(r'pub enum IconName \{(.*?)\}', content, re.DOTALL)
    if not enum_match:
        raise Exception("Could not find IconName enum")

    enum_content = enum_match.group(1)
    existing = set()
    for line in enum_content.strip().split('\n'):
        line = line.strip().rstrip(',')
        if line and not line.startswith('//'):
            existing.add(line)

    return existing, content

def get_new_icons(icons_ext_dir, existing_icons):
    """Find new icons in icons-ext directory."""
    new_icons = []

    for filename in sorted(os.listdir(icons_ext_dir)):
        if filename.endswith('.svg'):
            base_name = filename[:-4]  # Remove .svg
            enum_name = kebab_to_pascal(base_name)

            if enum_name not in existing_icons:
                new_icons.append((enum_name, base_name))

    return new_icons

def update_icon_rs(icon_rs_path, new_icons):
    """Update icon.rs with new icons."""
    with open(icon_rs_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # Find the enum declaration
    enum_start = None
    enum_end = None
    path_match_start = None
    path_match_end = None

    for i, line in enumerate(lines):
        if 'pub enum IconName {' in line:
            enum_start = i
        elif enum_start is not None and enum_end is None and line.strip() == '}':
            enum_end = i
        elif 'pub fn path(self) -> SharedString {' in line:
            path_match_start = i
        elif path_match_start is not None and path_match_end is None and line.strip() == '.into()':
            # Find the closing brace before .into()
            for j in range(i - 1, path_match_start, -1):
                if lines[j].strip() == '}':
                    path_match_end = j
                    break

    if not all([enum_start, enum_end, path_match_start, path_match_end]):
        raise Exception("Could not find required sections in icon.rs")

    # Add new enum variants (sorted)
    new_enum_lines = []
    for enum_name, _ in sorted(new_icons):
        new_enum_lines.append(f"    {enum_name},\n")

    # Add new path matches (sorted)
    new_path_lines = []
    for enum_name, file_name in sorted(new_icons):
        new_path_lines.append(f'            Self::{enum_name} => "icons/{file_name}.svg",\n')

    # Insert new lines
    lines = (
        lines[:enum_end] +
        new_enum_lines +
        lines[enum_end:path_match_end] +
        new_path_lines +
        lines[path_match_end:]
    )

    # Write back
    with open(icon_rs_path, 'w', encoding='utf-8') as f:
        f.writelines(lines)

def move_icons(icons_ext_dir, icons_dir, new_icons):
    """Move icons from icons-ext to icons directory."""
    os.makedirs(icons_dir, exist_ok=True)

    for _, file_name in new_icons:
        src = os.path.join(icons_ext_dir, f"{file_name}.svg")
        dst = os.path.join(icons_dir, f"{file_name}.svg")

        if os.path.exists(src):
            shutil.copy2(src, dst)
            print(f"Copied: {file_name}.svg")

def main():
    # Paths
    root_dir = Path(__file__).parent
    icons_ext_dir = root_dir / "assets" / "icons-ext"
    icons_dir = root_dir / "assets" / "icons"
    icon_rs_path = root_dir / "crates" / "ui" / "src" / "icon.rs"

    print("Scanning for new icons...")

    # Get existing icons
    existing_icons, _ = get_existing_icons(icon_rs_path)
    print(f"Found {len(existing_icons)} existing icons in icon.rs")

    # Get new icons
    new_icons = get_new_icons(icons_ext_dir, existing_icons)
    print(f"Found {len(new_icons)} new icons to add")

    if not new_icons:
        print("No new icons to add!")
        return

    print("\nNew icons to add:")
    for enum_name, file_name in new_icons[:10]:
        print(f"  - {enum_name} ({file_name}.svg)")
    if len(new_icons) > 10:
        print(f"  ... and {len(new_icons) - 10} more")

    # Update icon.rs
    print("\nUpdating icon.rs...")
    update_icon_rs(icon_rs_path, new_icons)
    print("Updated icon.rs")

    # Move icons
    print("\nCopying icons to assets/icons/...")
    move_icons(icons_ext_dir, icons_dir, new_icons)
    print("Copied all icons")

    print(f"\nSuccessfully added {len(new_icons)} new icons!")
    print("\nNote: Run 'cargo check' to verify the changes compile correctly.")

if __name__ == "__main__":
    main()
