// Phase 2 angle-reading aids. Units are millimetres; angles are degrees.
// Design rationale: docs/phase-2-protractor-spec.md, v2.3, especially its stack-up.
// Export one part with -D 'part="disc"', 'part="pointer"', or 'part="coupon"'.
// The default is a GUI layout: three separate parts, each resting on its print face.

/* [Selection] */
part = "all";                    // [all, disc, pointer, coupon, screws, screw_fit_test]
assembled_view = false;          // Only for the pointer side PNGs; leave false for STL.

/* [Motor: mounting plate is z = 0, shaft points toward +z] */
motor_boss_d = 9.0;              // The boss, rather than the loose screw holes, locates zero.
motor_boss_h = 1.5;
motor_shaft_d = 5.0;
motor_shaft_h = 10.0;
motor_flat_length = 6.0;        // Only the outer 6 mm of the shaft has the two flats.
motor_flat_nominal = 3.0;
ear_pitch = 35.0;               // Hole centres are x = +/-17.5, y = -8, NOT y = 0.
ear_y = -8.0;                   // The negative-y side is the cable side.
ear_hole_nominal_d = 4.2;

/* [Disc and locating holes] */
disc_d = 190.0;                 // Fits the Creality Hi's 260 mm bed, including a brim.
disc_h = 2.4;                   // Twelve 0.2 mm layers: enough stiffness without a thick rim.
boss_radial_clearance = 0.2;    // Ø9.4: leave room for FDM holes printing slightly undersize.
boss_hole_d = motor_boss_d + 2 * boss_radial_clearance;
mount_hole_d = 4.6;             // M4 screws clamp; these holes deliberately do not centre it.
mark_h = 0.4;                   // Two layers of raised ticks and numerals.
tick_rim = 2.0;
tick_outer_r = disc_d / 2 - tick_rim;
tick_step = 1;
medium_step = 5;
major_step = 10;
full_turn = 360;
minor_length = 3.0;
medium_length = 5.0;
major_length = 8.0;
zero_length = 10.0;
minor_width = 0.5;              // A dependable single line from a 0.4 mm nozzle.
medium_width = 0.8;             // Two nozzle widths make the 5° and 10° ticks distinct.
major_width = 0.8;
zero_width = 1.2;
label_size = 5.0;
label_inset = 16.0;
label_r = disc_d / 2 - label_inset; // r = 79 at the default diameter; follows disc_d.
label_font = "Liberation Sans:style=Bold"; // Bundled with OpenSCAD; no downloaded font needed.
index_width = 2.4;
index_depth = 1.6;
index_rim_gap = 0.2;
index_outer_r = disc_d / 2 - index_rim_gap;

/* [Pointer: use the coupon to choose the flat fit] */
press_hub_d = 14.0;
clamp_hub_d = 18.0;             // The nut needs more radial room than the press-fit hub.
round_bore_d = 5.3;             // Radial clearance belongs on the arcs, not on the flats.
flat_fit = 3.0;                 // v2 fit finding: uniform clearance permits degrees of rock.
hub_clamp = false;
hub_d = hub_clamp ? clamp_hub_d : press_hub_d;
arm_h = 2.4;
arm_root_width = 8.0;
arm_tip_width = 2.0;
arm_tip_inset = 5.0;
arm_tip_r = tick_outer_r - arm_tip_inset; // r = 88 by default.
tip_width = 0.8;                // Tangential width: two nozzle lines, not a fragile knife edge.
tip_root_overlap = 1.0;         // The tip piece's root reaches this far back inside the arm's taper.
tip_root_sink = 0.2;            // One layer inside the arm's top and bottom faces: no shared planes.
tip_root_step = 0.3;            // The root's sunk profile steps to full height just outside the arm.
tip_span = 3.0;
tip_outer_r = tick_outer_r;
arm_end_r = tip_outer_r - tip_span; // r = 90: the high neck ends before the visible end face.
tip_web_inner_r = arm_end_r - 1.0; // The vertical web under the arm spans r = 89 ... 90.
tip_bar_h = 0.8;                // Height of the exposed end face, not a flat run at the tip.
tip_gap = 0.1;                  // Tightest paper gap: compute all clearances at this height.
relief_margin = 0.7;            // Clones can have flats starting higher than nominal z = 4.
max_head_h = 3.0;               // Button heads fit; 4 mm socket-cap heads do not.
required_head_gap = 1.0;

/* [Optional M3 split clamp, on the side opposite the arm] */
clamp_slot_width = 0.8;         // Two extrusion widths let the two jaws actually move.
clamp_bolt_d = 3.3;             // Clearance for an M3 screw, not an M3 printed thread.
clamp_bolt_x = -(hub_d + round_bore_d) / 4; // Midway through the rear radial wall.
clamp_nut_flats = 5.8;          // M3's 5.5 mm nut plus FDM insertion clearance.
clamp_nut_sides = 6;
clamp_nut_r = clamp_nut_flats / (2 * cos(180 / clamp_nut_sides));
clamp_nut_floor = 0.2;          // One full layer under the nut; its top is open for printing.
clamp_bolt_z = clamp_nut_floor + clamp_nut_flats / 2;
clamp_jaw_wall = 1.6;
clamp_nut_thickness = 2.4;
clamp_jaw_inner_y = max(clamp_slot_width / 2, flat_fit / 2);
clamp_nut_inner_y = clamp_jaw_inner_y + clamp_jaw_wall;
// Measure the available pocket depth along the screw axis, where it meets the
// round hub wall. Using hub_d/2 alone would overstate it on this offset chord.
clamp_wall_outer_y = sqrt(pow(hub_d / 2, 2) - pow(clamp_bolt_x, 2));
clamp_nut_depth = clamp_wall_outer_y - clamp_nut_inner_y;
clamp_nut_jaw_wall = clamp_nut_inner_y - clamp_jaw_inner_y;
clamp_head_jaw_wall = clamp_wall_outer_y - clamp_jaw_inner_y;

/* [Coupon: one connected print, with accessible through bores] */
coupon_x = 70.0;
coupon_y = 34.0;
coupon_h = disc_h;
flat_series = [2.8, 2.9, 3.0, 3.1, 3.2];
coupon_ring_d = press_hub_d;    // Test the Ø14 press fit even if the clamp option is selected.
coupon_hole_gap = 2.0;
coupon_ring_pitch = (coupon_x - coupon_ring_d) / (len(flat_series) - 1);
coupon_ring_y = (coupon_ring_d + boss_hole_d) / 2 + coupon_hole_gap;
// The boss hole stays at the shaft origin. Move the PLATE toward +y instead:
// y = -13.3 ... 20.7 leaves a full 2 mm between the centre collar and boss hole,
// with all five Ø14 footprints inside the 70 x 34 plate. A centred plate would
// put its centre collar across the boss hole and soften that test fit.
coupon_min_y = coupon_ring_y + coupon_ring_d / 2 - coupon_y;
coupon_centres = [for (i = [0 : len(flat_series) - 1])
    [(i - (len(flat_series) - 1) / 2) * coupon_ring_pitch, coupon_ring_y]];
dot_d = 1.0;
dot_h = mark_h;
dot_r = 5.2;
dot_arc_start = 30;
dot_arc_end = 150;
label_pad_inner_r = 4.3;
label_pad_outer_r = 6.1;

/* [Substitute screws: printed bolt and nut for the two ear holes, until real M4 screws arrive] */
// These replace the M4 button-head screws. They do not have to be metric,
// because the bolt and its nut are printed together: what matters is that the
// bolt passes the motor's Ø4.2 steel ear hole, that its head stays inside the
// 3 mm keep-out under the pointer arm, and that the nut fits beside the can.
screw_major_d = 3.9;            // Outer thread diameter: passes the Ø4.2 ear and Ø4.6 disc holes.
screw_pitch = 1.5;              // Coarse on purpose: a 0.2 mm layer gives 7.5 layers per turn.
screw_thread_e = 0.3;           // Thread depth is twice this; see helix() for the eccentric profile.
screw_profile_r = screw_major_d / 2 - screw_thread_e;
screw_head_d = 9.0;
screw_head_h = 2.4;             // Inside max_head_h (3 mm): the arm passes 1.6 mm above it.
screw_length = 9.0;             // Disc 2.4 + ear (about 1) + nut 4 + spare; ears up to 2.6 mm fit.
screw_tip_chamfer = 0.6;        // A lead-in so the first turn finds the nut's thread.
screw_grip_notches = 18;        // Finger grip on the head rim; no screwdriver needed.
screw_grip_notch_d = 1.0;
nut_h = 4.0;
nut_d = 9.0;
nut_clearances = [0.2, 0.2, 0.35, 0.35]; // Two snug nuts, two looser spares (marked with a rim notch).
nut_entry_chamfer = 0.4;
motor_can_d = 28.0;             // The can is centred on the ear line, 8 mm off the shaft.
nut_can_gap = 0.3;
// Each ear hole is ear_pitch / 2 = 17.5 mm from the can's centre and the can's
// radius is 14, so only 3.5 mm separate the hole's centre from the can wall.
// A round Ø9 nut would hit the can. The flat side, turned toward the can, both
// clears it and stops the nut from spinning: you only turn the bolt's head.
nut_flat_offset = ear_pitch / 2 - motor_can_d / 2 - nut_can_gap; // 3.2 mm.
nut_min_wall = 0.8;             // Two extrusion lines between the thread and the flat.
nut_mark_d = 1.2;
thread_slice_h = 0.05;          // Extrusion slices: 30 per pitch keeps the helix smooth.
thread_fn = 48;
screw_layout_pitch = 11.0;      // Six separate parts on one plate, 2 mm apart.

/* [Numerical quality, print layout and preview colours] */
$fa = 0.5;                     // At r = 95, chord error is under 0.001 mm, below the 0.01 budget.
$fs = 0.3;
boolean_overlap = 0.02;        // Boolean overlap only; never added to an outside dimension.
comparison_epsilon = 1e-9;      // Assertions only: subtracted dimensions can round below their limits.
layout_gap = 12.0;
pointer_layout_x = disc_d / 2 + layout_gap + hub_d / 2;
coupon_layout_x = pointer_layout_x + tip_outer_r + layout_gap + coupon_x / 2;
body_colour = [0.88, 0.85, 0.76];
mark_colour = [0.12, 0.14, 0.16];
pointer_colour = [0.90, 0.43, 0.15];

/* [Derived assembly stack-up: NOT the upside-down print coordinates] */
ticks_top_z = disc_h + mark_h;
arm_bottom_z = disc_h + max_head_h + required_head_gap; // 6.4: 1 mm above a 3 mm head.
arm_top_z = arm_bottom_z + arm_h;         // 8.8; the nominal shaft tip leaves 1.2 exposed.
hub_bottom_z = ticks_top_z + tip_gap;     // 2.9, at the 0.1 mm paper gap.
hub_top_z = arm_top_z;
hub_h = hub_top_z - hub_bottom_z;         // Derived: 5.9.
// The shaft is round below z = 4: full-height bore flats would stop the hub there,
// before the tip reached its paper gap. Clear that length plus the clone margin;
// the shoulder in use is z = 4.7, leaving 4.1 mm of flats up to the arm's top.
bore_relief_h = (motor_shaft_h - motor_flat_length) - hub_bottom_z + relief_margin;
bore_engagement_h = hub_h - bore_relief_h; // 4.1 mm of flats above the 1.8 mm relief.
head_top_z = disc_h + max_head_h;          // 5.4 for a 3 mm button head.
// Use the actual print-space arm and tip faces, reflected into assembly z.
head_clearance = (hub_top_z - arm_h) - head_top_z;
tip_datum_print_z = hub_h;
tip_bar_top_print_z = tip_datum_print_z - tip_bar_h;
// A 45° slope reaches the end face directly, without a flat overhang at its end.
// It starts 0.3 mm beyond the neck's end, leaving a ledge at arm_h (r = 90 ... 90.3).
// Other parameters (max_head_h above 3.3, for one) would start it inside the neck,
// so clamp to the neck's end at 90 for a slightly steeper face.
tip_slope_start_r = max(arm_end_r, tip_outer_r - (tip_bar_top_print_z - arm_h));
tip_slope_overhang_deg = atan((tip_outer_r - tip_slope_start_r) /
                             (tip_bar_top_print_z - arm_h)); // From vertical in print.
tip_bottom_z = hub_top_z - tip_datum_print_z;
tip_clearance = tip_bottom_z - ticks_top_z;
coupon_top_z = coupon_h + hub_h;
// Lower the local label pad, then raise dots from its floor. The dots are 0.4 mm
// proud of that pad but stay inside the coupon's 8.3 mm height.
label_pad_z = coupon_top_z - dot_h;

assert(part == "all" || part == "disc" || part == "pointer" || part == "coupon" ||
       part == "screws" || part == "screw_fit_test",
       "part must be all, disc, pointer, coupon, screws or screw_fit_test");
assert(flat_fit > 0 && flat_fit < round_bore_d, "flat_fit must leave two flats");
assert(len(flat_series) == 5, "flat_series needs five candidates");
assert(bore_relief_h >= 0, "Round relief cannot have a negative height");
assert(bore_engagement_h >= 3.0, "Double-D engagement must be at least 3 mm");
echo(bore_relief_h_mm = bore_relief_h, double_d_engagement_mm = bore_engagement_h);

// A double-D is the INTERSECTION of a round hole and a parallel-sided strip.
// Subtracting this shape leaves the shaft's flats in contact even though the arc
// diameter has clearance. A single D would have only one of the two flats.
// The plate-facing end in use is the TOP in print coordinates, for both the
// upside-down pointer and the coupon collars tried free-end first. Widen only
// that end to a round Ø5.3 relief so it passes the shaft's unflattened base.
// Height is the actual part height; extend cutters past the faces without
// shifting the relief shoulder or changing the remaining double-D engagement.
module double_d_bore(across_flats, height) {
    union() {
        translate([0, 0, -boolean_overlap])
            intersection() {
                cylinder(d = round_bore_d, h = height + 2 * boolean_overlap);
                translate([-round_bore_d / 2, -across_flats / 2, 0])
                    cube([round_bore_d, across_flats, height + 2 * boolean_overlap]);
            }
        translate([0, 0, height - bore_relief_h])
            cylinder(d = round_bore_d, h = bore_relief_h + boolean_overlap);
    }
}

// Reuse exactly the same boss and ear-hole geometry on the big disc and coupon.
// Hole cutters extend past both faces so no coplanar membrane survives the CSG.
module mounting_holes(height) {
    translate([0, 0, -boolean_overlap])
        cylinder(d = boss_hole_d, h = height + 2 * boolean_overlap);
    for (side = [-1, 1])
        translate([side * ear_pitch / 2, ear_y, -boolean_overlap])
            cylinder(d = mount_hole_d, h = height + 2 * boolean_overlap);
}

function tick_length(angle) = angle == 0 ? zero_length :
    angle % major_step == 0 ? major_length :
    angle % medium_step == 0 ? medium_length : minor_length;

function tick_width(angle) = angle == 0 ? zero_width :
    angle % major_step == 0 ? major_width :
    angle % medium_step == 0 ? medium_width : minor_width;

module markings_2d() {
    // Local +y is the zero ray. A NEGATIVE OpenSCAD rotation moves it clockwise
    // when viewed from +z: 0 above the shaft, 10 to its right. Text rotates with
    // its ray, so the tops of the digits always point outward (never mirrored).
    for (angle = [0 : tick_step : full_turn - tick_step])
        rotate(-angle)
            translate([-tick_width(angle) / 2, tick_outer_r - tick_length(angle)])
                square([tick_width(angle), tick_length(angle)]);
    for (angle = [0 : major_step : full_turn - major_step])
        rotate(-angle)
            translate([0, label_r])
                text(str(angle), size = label_size, font = label_font,
                     halign = "center", valign = "center");
    // The small inward-pointing triangle distinguishes zero from every 10° tick
    // without extending the disc beyond Ø190 or consuming the reading interval.
    polygon([[-index_width / 2, index_outer_r],
             [0, index_outer_r - index_depth],
             [index_width / 2, index_outer_r]]);
}

module disc() {
    color(body_colour)
        difference() {
            cylinder(d = disc_d, h = disc_h);
            mounting_holes(disc_h);
        }
    // Start slightly INSIDE the base: each raised stroke is fused to it.
    color(mark_colour)
        translate([0, 0, disc_h - boolean_overlap])
            linear_extrude(height = mark_h + boolean_overlap) markings_2d();
}

module arm_2d() {
    // The arm proper is the 8 -> 2 mm taper and ends at r = 88. Everything
    // beyond it (the 0.8 mm neck, the web, the slope and the end face) belongs to
    // tip_wedge(), so no face of the arm is coplanar with a face of the tip.
    polygon([[0, -arm_root_width / 2],
             [arm_tip_r, -arm_tip_width / 2],
             [arm_tip_r, arm_tip_width / 2],
             [0, arm_root_width / 2]]);
}

module tip_wedge() {
    // Profile is (radius, print z); rotating +90° about x extrudes toward -y.
    // In use its lower face is the hub-bottom plane. Its upper face slopes at
    // 45° (never shallower) all the way to the 0.8 x 0.8 mm end face at r = 93.
    // Read that face against the outer ends of the ticks from above. In print
    // the slope is the lower boundary: at most 45° from vertical, with no flat
    // run or cantilever at the end, so no supports are needed.
    // The root (r = 87 ... 88) sits one layer inside the arm's top and bottom
    // faces and well inside its 2 mm width, so the union of the two solids is a
    // clean crossing. Two earlier versions fused them along shared faces and a
    // hairline sliver; Manifold then emitted duplicate vertices and zero-area
    // triangles that stl_bounds.py rejects. The 0.3 mm step to full height just
    // outside the arm is a 0.2 mm overhang that any slicer bridges.
    translate([0, tip_width / 2, 0])
        rotate([90, 0, 0])
            linear_extrude(height = tip_width)
                polygon([[arm_tip_r - tip_root_overlap, tip_root_sink],
                         [arm_tip_r + tip_root_step, tip_root_sink],
                         [arm_tip_r + tip_root_step, 0],
                         [arm_end_r, 0],
                         // Keep the ledge only when it has length: a clamped
                         // slope must not leave duplicate polygon vertices.
                         if (tip_slope_start_r > arm_end_r) [arm_end_r, arm_h],
                         [tip_slope_start_r, arm_h],
                         [tip_outer_r, tip_bar_top_print_z],
                         [tip_outer_r, tip_datum_print_z],
                         [tip_web_inner_r, tip_datum_print_z],
                         [tip_web_inner_r, arm_h],
                         [arm_tip_r + tip_root_step, arm_h],
                         [arm_tip_r + tip_root_step, arm_h - tip_root_sink],
                         [arm_tip_r - tip_root_overlap, arm_h - tip_root_sink]]);
}

module clamp_cutouts() {
    // Slot opens the rear of the bore to the outside. A tangential screw draws
    // the two jaws together; a radial screw would merely press on the shaft.
    translate([-hub_d / 2 - boolean_overlap, -clamp_slot_width / 2, -boolean_overlap])
        cube([hub_d / 2 + boolean_overlap, clamp_slot_width,
              hub_h + 2 * boolean_overlap]);
    translate([clamp_bolt_x, -hub_d / 2 - boolean_overlap, clamp_bolt_z])
        rotate([-90, 0, 0])
            cylinder(d = clamp_bolt_d, h = hub_d + 2 * boolean_overlap);
    // Hex flats are horizontal in print coordinates. The pocket reaches the
    // outside of the hub for side insertion; keep that opening free of support
    // material so the nut can slide all the way to its inner seating face.
    translate([clamp_bolt_x, clamp_nut_inner_y, clamp_bolt_z])
        rotate([-90, 0, 0])
            cylinder(r = clamp_nut_r, h = hub_d / 2 - clamp_nut_inner_y + boolean_overlap,
                     $fn = clamp_nut_sides);
}

module pointer() {
    assert(head_clearance + comparison_epsilon >= required_head_gap,
           "Pointer arm would hit the screw heads");
    assert(arm_h < tip_bar_top_print_z, "Tip wedge needs room to slope below the arm");
    assert(tip_slope_overhang_deg <= 45.5, "Tip slope must overhang at most 45.5 degrees from vertical");
    if (hub_clamp) {
        assert(clamp_nut_jaw_wall + comparison_epsilon >= clamp_jaw_wall &&
               clamp_head_jaw_wall + comparison_epsilon >= clamp_jaw_wall,
               "Both clamp jaws need 1.6 mm walls");
        assert(clamp_nut_depth >= clamp_nut_thickness, "Hub must contain the full 2.4 mm nut");
    }
    // v2 finding 1: never sweep a low arm through the heads at the ear radius.
    // v2.3 datum: read the exposed 0.8 x 0.8 mm END FACE at r = 93 from above,
    // against the outer ends of the ticks; the broad arm is never the datum.
    // These are assembly clearances, despite this module's reversed print z.
    echo(head_clearance_mm = head_clearance, tip_tick_clearance_mm = tip_clearance,
         tip_slope_overhang_from_vertical_deg = tip_slope_overhang_deg);
    color(pointer_colour)
        difference() {
            union() {
                cylinder(d = hub_d, h = hub_h);
                linear_extrude(height = arm_h) arm_2d();
                tip_wedge();
            }
            double_d_bore(flat_fit, hub_h);
            if (hub_clamp) clamp_cutouts();
        }
}

module label_pad_2d() {
    intersection() {
        difference() {
            circle(r = label_pad_outer_r);
            circle(r = label_pad_inner_r);
        }
        translate([-label_pad_outer_r, 0])
            square([2 * label_pad_outer_r, label_pad_outer_r]);
    }
}

module coupon() {
    for (fit = flat_series)
        assert(fit > 0 && fit < round_bore_d, "Each coupon fit must leave two flats");
    // Check the whole circular footprints, not just the double-D bores. Cutting
    // a locating hole into a ring wall would make that candidate flex too easily.
    for (centre = coupon_centres) {
        assert(norm(centre) >= (coupon_ring_d + boss_hole_d) / 2 + coupon_hole_gap,
               "Collars need 2 mm clearance from the boss hole");
        for (side = [-1, 1])
            assert(norm(centre - [side * ear_pitch / 2, ear_y]) >=
                   (coupon_ring_d + mount_hole_d) / 2 + coupon_hole_gap,
                   "Collars need 2 mm clearance from the ear holes");
    }
    color(body_colour)
        difference() {
            union() {
                difference() {
                    union() {
                        translate([-coupon_x / 2, coupon_min_y, 0])
                            cube([coupon_x, coupon_y, coupon_h]);
                        for (centre = coupon_centres)
                            translate([centre[0], centre[1], coupon_h - boolean_overlap])
                                cylinder(d = coupon_ring_d, h = hub_h + boolean_overlap);
                    }
                    // Recess only the OUTWARD half of each collar's label pad.
                    // The motor locating holes stay clear, as do all fit flats.
                    for (i = [0 : len(coupon_centres) - 1])
                        translate([coupon_centres[i][0], coupon_centres[i][1], label_pad_z])
                            linear_extrude(height = dot_h + boolean_overlap)
                                label_pad_2d();
                }
                for (i = [0 : len(coupon_centres) - 1])
                    for (dot = [0 : i])
                        let(angle = i == 0 ? (dot_arc_start + dot_arc_end) / 2 :
                                dot_arc_start + dot * (dot_arc_end - dot_arc_start) / i)
                            translate([coupon_centres[i][0] + dot_r * cos(angle),
                                       coupon_centres[i][1] + dot_r * sin(angle),
                                       label_pad_z - boolean_overlap])
                                cylinder(d = dot_d, h = dot_h + boolean_overlap);
            }
            mounting_holes(coupon_top_z);
            // Through the plate too: a blind coupon hole cannot be pressed onto
            // the shaft. Invert the coupon to try the collar end first, using the
            // shaft's outer 6 mm; use the plate-down face for the mounting test.
            for (i = [0 : len(coupon_centres) - 1])
                translate([coupon_centres[i][0], coupon_centres[i][1], 0])
                    double_d_bore(flat_series[i], coupon_top_z);
        }
}

// ---------------------------------------------------------------------------
// Substitute screws.
//
// The thread is an "eccentric circle" helix: a circle of radius r, moved off the
// axis by screw_thread_e, extruded upward while it rotates one full turn per
// pitch. Every horizontal slice is a plain circle, which FDM prints well, and the
// slices stack into a single-start, rounded thread of depth 2 * e. A nut cut with
// the same helix but a slightly larger circle mates with it: at any height the
// bolt's circle sits inside the nut's circle with `c` of gap all round.
//
// The phase of the helix depends only on the absolute z, so any part built with
// helix() at any height belongs to the same screw. The bolt cannot be pulled out
// of the nut axially: shifting it by dz without turning rotates its circle by
// 360 * dz / pitch against the nut's, and the two circles stop overlapping once
// 2 * e * sin(angle / 2) exceeds c, which is why each clearance must stay below
// 2 * e.
//
// OpenSCAD's twist follows the left-hand rule, so a NEGATIVE twist makes the
// usual right-hand thread: turn clockwise, seen from the head, to tighten.
function thread_phase(z) = 360 * z / screw_pitch;

module helix(r, z0, h) {
    translate([0, 0, z0])
        rotate(thread_phase(z0))
            linear_extrude(height = h, twist = -360 * h / screw_pitch,
                           slices = ceil(h / thread_slice_h))
                translate([screw_thread_e, 0])
                    circle(r = r, $fn = thread_fn);
}

// Print coordinates: head on the bed (its underside is the face that bears on
// the disc), thread standing up. Upright is the only orientation that prints
// the thread's flanks as layers rather than as a fragile overhang.
module screw_bolt() {
    thread_top_z = screw_head_h + screw_length;
    difference() {
        cylinder(d = screw_head_d, h = screw_head_h);
        for (i = [0 : screw_grip_notches - 1])
            rotate(i * 360 / screw_grip_notches)
                translate([screw_head_d / 2, 0, -boolean_overlap])
                    cylinder(d = screw_grip_notch_d, h = screw_head_h + 2 * boolean_overlap,
                             $fn = 12);
    }
    intersection() {
        helix(screw_profile_r, screw_head_h - boolean_overlap, screw_length + boolean_overlap);
        // Tip chamfer: a cone that narrows the last 0.6 mm by 0.6 mm in radius.
        union() {
            cylinder(d = screw_major_d + 1, h = thread_top_z - screw_tip_chamfer);
            translate([0, 0, thread_top_z - screw_tip_chamfer])
                cylinder(d1 = screw_major_d + 1, d2 = screw_major_d + 1 - 2 * (screw_tip_chamfer + 0.5),
                         h = screw_tip_chamfer);
        }
    }
}

// The flat is on the nut's -x side. In use, turn that flat toward the motor can.
module screw_nut(c) {
    hole_r = screw_profile_r + c + screw_thread_e; // Outer envelope of the threaded hole.
    difference() {
        intersection() {
            cylinder(d = nut_d, h = nut_h);
            translate([-nut_flat_offset, -nut_d / 2, -boolean_overlap])
                cube([nut_d, nut_d, nut_h + 2 * boolean_overlap]);
        }
        helix(screw_profile_r + c, -boolean_overlap, nut_h + 2 * boolean_overlap);
        // Entry chamfers on both faces, so either face can go on first.
        for (z = [0, nut_h])
            translate([0, 0, z])
                mirror([0, 0, z == 0 ? 0 : 1])
                    translate([0, 0, -boolean_overlap])
                        cylinder(r1 = hole_r + nut_entry_chamfer, r2 = hole_r - 0.1,
                                 h = nut_entry_chamfer + 0.1 + boolean_overlap);
        // The looser spares carry a notch on the rim opposite the flat.
        if (c > nut_clearances[0])
            translate([nut_d / 2, 0, -boolean_overlap])
                cylinder(d = nut_mark_d, h = nut_h + 2 * boolean_overlap, $fn = 12);
    }
}

module screws() {
    assert(screw_head_h <= max_head_h, "Screw head taller than the keep-out under the pointer arm");
    assert(screw_major_d < ear_hole_nominal_d && screw_major_d < mount_hole_d,
           "Bolt must pass the ear and disc holes");
    for (c = nut_clearances) {
        assert(c > 0 && c < 2 * screw_thread_e, "Each nut clearance must stay below the thread depth");
        assert(nut_flat_offset - (screw_profile_r + c + screw_thread_e) >= nut_min_wall,
               "Nut wall at the flat is too thin");
    }
    echo(screw_head_top_above_disc_mm = screw_head_h,
         arm_clearance_above_screw_head_mm = arm_bottom_z - disc_h - screw_head_h,
         nut_flat_to_can_mm = nut_can_gap,
         nut_wall_at_flat_mm = nut_flat_offset - (screw_profile_r + nut_clearances[0] + screw_thread_e));
    for (i = [0 : 1])
        translate([i * screw_layout_pitch, 0, 0]) screw_bolt();
    for (i = [0 : len(nut_clearances) - 1])
        translate([(i + 2) * screw_layout_pitch, 0, 0]) screw_nut(nut_clearances[i]);
}

// Verification only, never printed: the snug nut screwed onto the bolt at a
// height that is a whole number of pitches, where both helices have the same
// phase. Rendering the intersection must produce nothing (no interference);
// fit_test_c below zero, or an offset that is not a whole pitch, must not.
fit_test_c = nut_clearances[0];
fit_test_offset = 3 * screw_pitch;
module screw_fit_test() {
    intersection() {
        screw_bolt();
        translate([0, 0, fit_test_offset]) screw_nut(fit_test_c);
    }
}

if (part == "disc") disc();
else if (part == "pointer") {
    if (assembled_view)
        translate([0, 0, hub_top_z]) mirror([0, 0, 1]) pointer();
    else pointer();
} else if (part == "coupon") coupon();
else if (part == "screws") screws();
else if (part == "screw_fit_test") screw_fit_test();
else {
    disc();
    translate([pointer_layout_x, 0, 0]) pointer();
    translate([coupon_layout_x, 0, 0]) coupon();
    translate([-2.5 * screw_layout_pitch, -disc_d / 2 - layout_gap - nut_d / 2, 0]) screws();
}
