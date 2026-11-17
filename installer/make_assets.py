from PIL import Image, ImageDraw, ImageFont
import os

out = r"D:\dsh\odm\installer"
os.makedirs(out, exist_ok=True)
font = ImageFont.truetype(r"C:\Windows\Fonts\segoeuib.ttf", 22)
small = ImageFont.truetype(r"C:\Windows\Fonts\segoeui.ttf", 12)


def gradient(w, h):
    img = Image.new("RGB", (w, h), (20, 70, 130))
    d = ImageDraw.Draw(img)
    for y in range(h):
        t = y / max(1, h - 1)
        d.line([(0, y), (w, y)], fill=(int(24 + 10 * t), int(88 - 20 * t), int(158 - 40 * t)))
    return img, d


img, d = gradient(493, 58)
d.text((18, 17), "Open Download Manager", font=font, fill=(255, 255, 255))
d.text((300, 24), "free download manager", font=small, fill=(200, 220, 245))
img.save(os.path.join(out, "banner.bmp"))

img, d = gradient(493, 312)
d.rectangle([0, 0, 492, 311], outline=(10, 40, 80))
d.text((36, 42), "Open Download Manager", font=font, fill=(255, 255, 255))
d.text((36, 80), "Segmented downloads, queues, scheduler,", font=small, fill=(210, 225, 245))
d.text((36, 100), "browser integration and more.", font=small, fill=(210, 225, 245))
d.text((36, 140), "Version 1.0.0", font=small, fill=(190, 210, 235))
d.polygon([(380, 70), (440, 70), (410, 130)], fill=(60, 200, 70))
d.rectangle([396, 60, 424, 150], fill=(60, 200, 70))
img.save(os.path.join(out, "dialog.bmp"))
print("images ok")
