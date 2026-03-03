# 6. JSON-Schnittstelle

## Schema-Export

```bash
photosolver schema -o schema.json
```

Generiert automatisch aus Pydantic-Modellen.

---

## Input Format (Rust → Python)

**Wichtig:** Kein `path` - nur Metadaten!

```json
{
  "photos": [
    {
      "id": "photo_001",
      "width": 4032,
      "height": 3024,
      "timestamp": "2024-07-15T14:30:00",
      "group": "2024-07-15_Urlaub",
      "area_weight": 1.0
    },
    {
      "id": "photo_002",
      "width": 3024,
      "height": 4032,
      "timestamp": "2024-07-15T15:45:00",
      "group": "2024-07-15_Urlaub",
      "area_weight": 2.0
    }
  ],
  "config": {
    "page_width_mm": 297.0,
    "page_height_mm": 210.0,
    "margin_mm": 10.0,
    "gap_mm": 3.0,
    "max_photos_per_page": 4,
    "target_pages": 20,
    "max_aspect_deviation": 0.2,
    "timeout_seconds": 30,
    "weight_aspect_ratio": 1.0,
    "weight_group_cohesion": 2.0,
    "weight_page_count": 0.5
  }
}
```

---

## Output Format (Python → Rust)

```json
{
  "pages": [
    {
      "page_number": 1,
      "placements": [
        {
          "photo_id": "photo_001",
          "x_mm": 10.0,
          "y_mm": 10.0,
          "width_mm": 135.5,
          "height_mm": 190.0
        },
        {
          "photo_id": "photo_002",
          "x_mm": 148.5,
          "y_mm": 10.0,
          "width_mm": 138.5,
          "height_mm": 190.0
        }
      ]
    }
  ],
  "statistics": {
    "status": "OPTIMAL",
    "solve_time_seconds": 2.45,
    "num_photos": 47,
    "num_pages": 12
  }
}
```

---

## Validierung

```bash
# Input validieren
photosolver validate input.json

# Output:
# ✅ Valid input: 47 photos
#    Config: 297.0x210.0mm
```

---

## Rekonstruktion in Rust

Rust hält intern `HashMap<photo_id, Photo>` mit paths:

```rust
fn reconstruct_pages(
    api_pages: Vec<ApiPage>,
    photo_map: &HashMap<String, Photo>
) -> Result<Vec<Page>> {
    api_pages
        .into_iter()
        .map(|page| {
            // photo_id → Full Photo (mit path)
            reconstruct_placements(page, photo_map)
        })
        .collect()
}
```

➡️ [7. Optimierung](7_optimierung.md)
