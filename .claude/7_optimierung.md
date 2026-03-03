# 7. Optimierungsziele & Constraints

> 📘 **Mathematische Formulierung:** Siehe [`python/docs/solver_model.typ`](../../python/docs/solver_model.typ)  
> Detaillierte CSP-Modellierung mit OR-Tools Constraints

## Hard Constraints (MÜSSEN erfüllt sein)

### 1. Chronologische Reihenfolge
- Foto mit früherer Timestamp kommt auf früherer/gleicher Seite
- Keine Umordnung innerhalb Gruppen
- **Gruppen-Ordnung:** Ergibt sich aus **lexikalischer Sortierung der Gruppennamen**
  - Beispiel: `2024-07-15_Urlaub` vor `2024-08-20_Geburtstag`

### 2. Keine Überlappung
- Photos dürfen sich nicht überlappen (2D NoOverlap)
- **Implementation:** `AddNoOverlap2D` (OR-Tools)
- **Diskretisierung:** 1mm
- Respektiere Ränder und Gaps

### 3. Gruppen-Angrenzung
- Verschiedene Gruppen dürfen nur gemischt werden, wenn chronologisch angrenzend
- **Constraint:** `max(x_start in Gruppe_A) < min(x_start in Gruppe_B)`
  - Rechtester oberer linker Startpunkt von A weiter links als linkester von B
- **Multi-Page:** Gruppen dürfen über mehrere Seiten gehen (bei großen Gruppen notwendig)

---

## Soft Constraints (Objective minimieren)

### 1. Aspect-Ratio-Abweichung
- Minimiere Differenz zwischen Original- und Platzierungs-Aspect-Ratio
- **Cropping:** Bevorzugt (Bild beschneiden statt schwarze Balken)
- **Letterboxing:** NEIN (wären schwarze Balken wie im Kino)
- **Max. Abweichung:** CLI-Parameter `--max-aspect-deviation` (default: 0.2 = ±20%)
- **Wichtig:** Input-Foto-Dimensionen werden NICHT respektiert
  - Fotos kommen groß rein, Solver bestimmt individuelle Zielgröße
  - Basierend auf: Seitengröße, Anzahl Bilder, area_weight
- **Gewichtung:** Mittel (default, aber CLI-anpassbar)

### 2. Area-Weight Zuweisung ✨ NEU
- Fotos mit höherem `area_weight` sollen mehr Fläche bekommen
- **Beispiel:** 3 Fotos mit [1, 1, 2]
  - Foto 1: ~25% der Fläche (1/4)
  - Foto 2: ~25% der Fläche (1/4)
  - Foto 3: ~50% der Fläche (2/4)
- **Berechnung:** `target_area_i = usable_area * (weight_i / sum_weights_on_page)`
- **Wichtig:** Gilt nur für Fotos auf **derselben Seite**

**Implementation:**
- **Soft-Constraint mit starker Bestrafung**
- **Toleranz:** ±25% auf die Fläche ist akzeptabel
- **Bei Konflikten:** Wenn Problem unlösbar → klare Fehlermeldung zurück
- **Gewichtung:** Hoch (default, aber CLI-anpassbar)

### 3. Gruppen-Kohäsion
- Bevorzuge Gruppen komplett auf einer Seite
- Penalty für Gruppen-Splits
- **Gewichtung:** Hoch (default, aber CLI-anpassbar)

### 4. Seitenzahl-Ziel
- Wenn `target_pages` gesetzt: bevorzuge diese Anzahl
- Penalty für Abweichung
- **Gewichtung:** Niedrig (default, aber CLI-anpassbar)

### 5. Ästhetische Balance
- Bevorzuge ausgeglichene Layouts
- **Gewichtung:** Niedrig (default, aber CLI-anpassbar)

---

## OR-Tools Implementation (TODO)

```python
# Objective Function (ALLE Gewichtungen via CLI/API anpassbar)
objective = (
    config.weight_aspect_ratio * aspect_ratio_deviation +
    config.weight_area * area_weight_penalty +  # HOCH (starke Bestrafung)
    config.weight_group_cohesion * group_split_penalty +
    config.weight_page_count * page_count_deviation
)

model.Minimize(objective)
```

### Area-Weight Constraint (Pseudo-Code)

```python
for page in pages:
    photos_on_page = [p for p in photos if assigned_to(p, page)]
    total_weight = sum(p.area_weight for p in photos_on_page)
    usable_area = (page_width - 2*margin) * (page_height - 2*margin)
    
    for photo in photos_on_page:
        target_area = usable_area * (photo.area_weight / total_weight)
        actual_area = photo.placed_width * photo.placed_height
        
        # Soft-Constraint: Toleranz ±25%
        lower_bound = target_area * 0.75
        upper_bound = target_area * 1.25
        
        if actual_area < lower_bound or actual_area > upper_bound:
            penalty = abs(actual_area - target_area)
            deviation += config.weight_area * penalty  # Starke Bestrafung

# Bei Konflikten (UNSAT/INFEASIBLE):
# → Fehlermeldung: "No solution found. Try relaxing constraints:
#    - Increase max_aspect_deviation
#    - Reduce area_weight differences
#    - Increase target_pages"
```

➡️ [8. Workflow](8_workflow.md)
