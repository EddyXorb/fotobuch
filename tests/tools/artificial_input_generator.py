#!/usr/bin/env python3
"""
Artificial Input Generator für Photobook Solver Tests

Generiert Dummy-Fotos mit zufälligen Farben und Größen, gruppiert in Ordnern.
"""

from pathlib import Path
from typing import Annotated
import random
from datetime import datetime, timedelta

import typer
from PIL import Image, ImageDraw, ImageFont

app = typer.Typer(help="Generiert Test-Fotos für Photobook Solver")


def generate_random_color() -> tuple[int, int, int]:
    """Generiert eine zufällige RGB-Farbe."""
    return (
        random.randint(50, 255),
        random.randint(50, 255),
        random.randint(50, 255),
    )


def generate_random_size() -> tuple[int, int]:
    """Generiert zufällige Foto-Dimensionen.
    
    Returns:
        (width, height) in Pixel
    """
    aspect_ratios = [
        (4, 3),    # Landscape
        (3, 4),    # Portrait
        (16, 9),   # Wide
        (9, 16),   # Tall
        (1, 1),    # Square
        (3, 2),    # Classic
    ]
    
    ratio = random.choice(aspect_ratios)
    base_size = random.randint(2000, 4500)
    
    if ratio[0] > ratio[1]:  # Landscape/Wide
        width = base_size
        height = int(base_size * ratio[1] / ratio[0])
    else:  # Portrait/Tall/Square
        height = base_size
        width = int(base_size * ratio[0] / ratio[1])
    
    return (width, height)


def create_dummy_photo(
    output_path: Path,
    label: str,
    width: int,
    height: int,
    color: tuple[int, int, int],
) -> None:
    """Erstellt ein Dummy-Foto mit einfarbigem Hintergrund und Label.
    
    Args:
        output_path: Pfad zur Ausgabe-Datei
        label: Text der in die Mitte geschrieben wird
        width: Breite in Pixel
        height: Höhe in Pixel
        color: RGB-Farbe als Tupel
    """
    img = Image.new('RGB', (width, height), color=color)
    draw = ImageDraw.Draw(img)
    
    # Label zentriert zeichnen
    font_size = min(width, height) // 15
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", font_size)
    except OSError:
        try:
            font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", font_size)
        except OSError:
            font = ImageFont.load_default()
    
    # Text-Bounding-Box berechnen
    bbox = draw.textbbox((0, 0), label, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    
    # Zentrieren
    x = (width - text_width) // 2
    y = (height - text_height) // 2
    
    # Text mit Kontrastfarbe
    text_color = (255, 255, 255) if sum(color) < 400 else (0, 0, 0)
    draw.text((x, y), label, fill=text_color, font=font)
    
    # Als JPEG speichern
    img.save(output_path, 'JPEG', quality=85)


@app.command()
def generate(
    num_groups: Annotated[int, typer.Option("--groups", "-g", help="Anzahl der Gruppen")] = 3,
    min_photos: Annotated[int, typer.Option("--min", help="Min. Fotos pro Gruppe")] = 3,
    max_photos: Annotated[int, typer.Option("--max", help="Max. Fotos pro Gruppe")] = 8,
    output: Annotated[Path, typer.Option("--output", "-o", help="Ausgabe-Verzeichnis")] = Path("test_photos_generated"),
    seed: Annotated[int | None, typer.Option("--seed", "-s", help="Random seed für Reproduzierbarkeit")] = None,
    with_timestamps: Annotated[bool, typer.Option("--with-timestamps", help="Dateinamen mit Timestamps (1s Abstand) versehen")] = False,
) -> None:
    """Generiert Test-Fotos mit zufälligen Farben und Größen.
    
    Mit --seed können reproduzierbare Test-Daten generiert werden.
    
    Beispiel:
        python artificial_input_generator.py generate --groups 5 --min 4 --max 10 -o my_test_data --seed 42
    """
    if min_photos > max_photos:
        typer.echo("❌ Fehler: --min muss <= --max sein", err=True)
        raise typer.Exit(1)
    
    if num_groups < 1:
        typer.echo("❌ Fehler: Mindestens 1 Gruppe erforderlich", err=True)
        raise typer.Exit(1)
    
    # Random seed setzen für Reproduzierbarkeit
    if seed is not None:
        random.seed(seed)
        typer.echo(f"🎲 Random seed: {seed}")
    
    # Ausgabe-Verzeichnis erstellen
    output.mkdir(parents=True, exist_ok=True)
    
    typer.echo(f"📸 Generiere {num_groups} Gruppen mit {min_photos}-{max_photos} Fotos...")
    typer.echo(f"📁 Ausgabe: {output.absolute()}")
    typer.echo()
    
    base_date = datetime(2024, 1, 1)
    total_photos = 0
    photo_timestamp = datetime(2024, 1, 1, 10, 10, 10)

    for group_idx in range(num_groups):
        # Gruppenname: Datum + Thema (lexikalisch sortierbar)
        group_date = base_date + timedelta(days=group_idx * 30)
        themes = ["Urlaub", "Geburtstag", "Hochzeit", "Ausflug", "Festival",
                  "Wanderung", "Strand", "Stadt", "Familie", "Party"]
        theme = random.choice(themes)
        group_name = f"{group_date.strftime('%Y-%m-%d')}_{theme}"

        # Gruppen-Ordner erstellen
        group_dir = output / group_name
        group_dir.mkdir(exist_ok=True)

        # Anzahl Fotos für diese Gruppe
        num_photos = random.randint(min_photos, max_photos)

        typer.echo(f"  📂 Gruppe {group_idx + 1}/{num_groups}: {group_name} ({num_photos} Fotos)")

        for photo_idx in range(num_photos):
            # Foto-Eigenschaften
            width, height = generate_random_size()
            color = generate_random_color()
            label = f"{group_name}\nFoto {photo_idx + 1}/{num_photos}\n{width}×{height}"

            # Dateiname mit fortlaufender Nummer (optional mit Timestamp)
            base_name = f"IMG_{(group_idx * 100 + photo_idx + 1):04d}"
            if with_timestamps:
                ts = photo_timestamp.strftime("%Y-%m-%d@%H%M%S")
                filename = f"{ts}_{base_name}.jpg"
                photo_timestamp += timedelta(days=1)
            else:
                filename = f"{base_name}.jpg"
            output_path = group_dir / filename
            
            # Foto erstellen
            create_dummy_photo(output_path, label, width, height, color)
            total_photos += 1
        
        typer.echo(f"    ✅ {num_photos} Fotos erstellt")
    
    typer.echo()
    typer.echo(f"✨ Fertig! {total_photos} Fotos in {num_groups} Gruppen generiert.")
    typer.echo(f"📂 Verzeichnis: {output.absolute()}")
    typer.echo()
    typer.echo("🚀 Testen mit:")
    typer.echo(f"   cargo run -- --input {output}")


@app.command()
def info() -> None:
    """Zeigt Informationen über den Generator."""
    typer.echo("📸 Artificial Input Generator")
    typer.echo()
    typer.echo("Generiert Dummy-Fotos für Photobook Solver Tests:")
    typer.echo("  • Zufällige RGB-Farben")
    typer.echo("  • Zufällige Größen (verschiedene Aspect-Ratios)")
    typer.echo("  • Gruppierung in Ordnern (lexikalisch sortierbar)")
    typer.echo("  • Labels zur Identifikation")
    typer.echo()
    typer.echo("Aspect-Ratios:")
    typer.echo("  • 4:3, 3:4 (klassisch)")
    typer.echo("  • 16:9, 9:16 (wide/tall)")
    typer.echo("  • 1:1 (square)")
    typer.echo("  • 3:2 (classic)")
    typer.echo()
    typer.echo("Verwendung:")
    typer.echo("  python artificial_input_generator.py generate --help")


if __name__ == "__main__":
    app()
