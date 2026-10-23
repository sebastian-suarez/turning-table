# Run from this directory, or use make -C hardware/cad from the repository root.
OPENSCAD ?= openscad
PYTHON ?= python3
SCAD_ARGS ?=

SOURCE := protractor.scad
OUT := ../measure
STLS := $(OUT)/protractor-disc.stl $(OUT)/pointer.stl $(OUT)/fit-coupon.stl
PNGS := $(OUT)/protractor-disc.png $(OUT)/pointer.png $(OUT)/pointer-tip.png
STL_FLAGS := --backend=manifold --export-format binstl --hardwarnings
PNG_FLAGS := --backend=manifold --hardwarnings --projection=o

.PHONY: all check check-recessed preview clean
.DELETE_ON_ERROR:

all: $(STLS)

$(OUT):
	mkdir -p "$@"

$(OUT)/protractor-disc.stl: $(SOURCE) Makefile | $(OUT)
	$(OPENSCAD) $(STL_FLAGS) $(SCAD_ARGS) -D 'part="disc"' -o "$@" "$(SOURCE)"

$(OUT)/pointer.stl: $(SOURCE) Makefile | $(OUT)
	$(OPENSCAD) $(STL_FLAGS) $(SCAD_ARGS) -D 'part="pointer"' -o "$@" "$(SOURCE)"

$(OUT)/fit-coupon.stl: $(SOURCE) Makefile | $(OUT)
	$(OPENSCAD) $(STL_FLAGS) $(SCAD_ARGS) -D 'part="coupon"' -o "$@" "$(SOURCE)"

# Bounds are the v2.3 acceptance defaults, with a 0.05 mm tolerance. Custom disc
# diameters can be checked with stl_bounds.py directly and their intended size.
check: all
	@$(PYTHON) stl_bounds.py --expect-size 190 190 2.8 --expect-min -95 -95 0 $(OUT)/protractor-disc.stl
	@$(PYTHON) stl_bounds.py --expect-size 100 14 5.9 --expect-min -7 -7 0 $(OUT)/pointer.stl
	@$(PYTHON) stl_bounds.py --max-size 70 34 8.3 --expect-min -35 -13.3 0 $(OUT)/fit-coupon.stl

# Recessed ticks need their matching, taller hub and coupon. Export all three in isolation;
# a style check must never replace the raised STLs waiting to go to the slicer.
# One shell keeps the scratch path and cleanup trap alive through every command.
check-recessed:
	@set -eu; \
	scratch=$$(mktemp -d "$${TMPDIR:-/tmp}/protractor-recessed.XXXXXX"); \
	trap 'rm -rf "$$scratch"' 0; \
	trap 'exit 1' 1 2 15; \
	$(OPENSCAD) $(STL_FLAGS) $(SCAD_ARGS) -D 'tick_style="recessed"' -D 'part="disc"' -o "$$scratch/protractor-disc.stl" "$(SOURCE)"; \
	$(PYTHON) stl_bounds.py --expect-size 190 190 2.4 --expect-min -95 -95 0 "$$scratch/protractor-disc.stl"; \
	$(OPENSCAD) $(STL_FLAGS) $(SCAD_ARGS) -D 'tick_style="recessed"' -D 'part="pointer"' -o "$$scratch/pointer.stl" "$(SOURCE)"; \
	$(PYTHON) stl_bounds.py --expect-size 100 14 6.3 --expect-min -7 -7 0 "$$scratch/pointer.stl"; \
	$(OPENSCAD) $(STL_FLAGS) $(SCAD_ARGS) -D 'tick_style="recessed"' -D 'part="coupon"' -o "$$scratch/fit-coupon.stl" "$(SOURCE)"; \
	$(PYTHON) stl_bounds.py --max-size 70 34 8.7 --expect-min -35 -13.3 0 "$$scratch/fit-coupon.stl"

preview: $(PNGS)

# Orthographic, exactly from +z: no perspective can reverse the clockwise scale.
# Preview colours distinguish the raised marks without changing the STL solids.
$(OUT)/protractor-disc.png: $(SOURCE) Makefile | $(OUT)
	$(OPENSCAD) $(PNG_FLAGS) $(SCAD_ARGS) -D 'part="disc"' --autocenter --viewall --camera=0,0,0,0,0,0,300 --imgsize=1800,1800 -o "$@" "$(SOURCE)"

# Show the pointer in USE orientation here, with the arm above the hub and its
# tip wedge down. The STL and default GUI layout remain upside down for printing.
# A fixed camera fills this wide image; automatic framing would shrink the tip.
$(OUT)/pointer.png: $(SOURCE) Makefile | $(OUT)
	$(OPENSCAD) $(PNG_FLAGS) $(SCAD_ARGS) -D 'part="pointer"' -D assembled_view=true --camera=43,0,6.1,90,0,0,60 --imgsize=1800,450 -o "$@" "$(SOURCE)"

# The last 6 mm show the neck, web and slope meeting the 0.8 mm end face.
$(OUT)/pointer-tip.png: $(SOURCE) Makefile | $(OUT)
	$(OPENSCAD) $(PNG_FLAGS) $(SCAD_ARGS) -D 'part="pointer"' -D assembled_view=true --camera=90,0,4.5,90,0,0,16 --imgsize=1200,600 -o "$@" "$(SOURCE)"

clean:
	rm -f $(STLS) $(PNGS)
