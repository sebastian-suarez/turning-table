// Phase 3 bench rig for W9/W10: a stand that holds the platter motor shaft-up, and a
// Ø150 test platter pressed straight onto the shaft. Units are millimetres.
// Export one part with -D 'part="base"' or 'part="platter"'; 'part="assembly"' shows
// both in use around a dummy motor (preview only). The default is a GUI layout.
// Not the Phase 4 platter: that one rides on two 608ZZ bearings so the shaft carries no
// weight. Here the shaft carries everything, so keep the object light and centred.

/* [Selection] */
part = "all";                   // [all, base, platter, assembly]

/* [Motor: mounting plate top is z = 0, shaft on the z axis pointing toward +z] */
// Same 28BYJ-48 geometry as protractor.scad; can and cover sizes from the datasheet.
motor_boss_d = 9.0;
motor_boss_h = 1.5;
motor_shaft_d = 5.0;
motor_shaft_h = 10.0;           // Shaft tip above the mounting plate.
motor_flat_length = 6.0;        // Only the outer 6 mm of the shaft has the two flats.
ear_pitch = 35.0;               // Ear holes at x = +/-17.5, y = -8.
ear_y = -8.0;                   // Also the can's centre: the shaft is 8 mm off it.
ear_r = 3.5;                    // 42 mm across the ears.
plate_h = 1.0;                  // The mounting plate with the ears, on top of the can.
can_d = 28.0;
can_h = 19.0;                   // Below the mounting plate.
cable_cover_w = 15.0;           // The wire cover on the can's -y side.

/* [Shaft fit: the values verified on this motor in protractor.scad, 2026-10-26] */
round_bore_d = 5.3;             // Clearance on the arcs; the flats carry the drive.
flat_fit = 3.0;                 // Passed the twist and rock checks on the pointer.
relief_margin = 0.7;            // Clones can have flats starting higher than nominal.

/* [Base] */
base_h = 24.0;                  // The can hangs from its ears, about 4 mm above the table.
tower_r = 28.0;                 // Top radius around the shaft: 5 mm of wall past the ears.
foot_r = 50.0;                  // Ø100 footprint, centred under the shaft and platter.
can_clearance = 0.5;            // Radial. The can only needs to pass; the ears locate.
plate_clearance = 0.4;          // Around the plate outline: about +/-1.3° of motor twist.
recess_h = 1.2;                 // Deeper than the plate, so the ears rest on the floor.
cable_slot_w = 18.0;            // Room for the 15 mm wire cover and the cable.

/* [Platter] */
platter_d = 150.0;
platter_h = 3.0;
hub_d = 14.0;                   // Same as the pointer's press-fit hub.
hub_gap = 1.5;                  // Hub above the boss once the shaft tip bottoms in the bore.
fillet_d = 34.0;                // Cone from the hub into the disc, against bending.
fillet_h = 4.0;
groove_w = 1.0;                 // Centring rings and cross, cut into the top face.
groove_h = 0.6;
centre_rings_r = [25, 50];
cross_l = 10.0;
notch_depth = 3.0;              // Rim index notch at +x, to watch whole revolutions.
notch_w = 4.0;

/* [Hidden] */
boolean_overlap = 0.01;
layout_gap = 10.0;
$fn = 128;

// Assembly stack-up, z above the motor's mounting plate. The bore is blind and as deep
// as the shaft is long, so pushing the platter on until the tip bottoms sets its height.
hub_bottom_z = motor_boss_h + hub_gap;    // 3.0
hub_h = motor_shaft_h - hub_bottom_z;     // 7.0: the hub's free end to the disc.
platter_bottom_z = motor_shaft_h;         // 10.0: the disc starts at the shaft tip.
// The shaft is round below z = 4: a round relief at the hub's free end passes that part,
// and the double-D grips the flats above it (protractor.scad does the same).
bore_relief_h = (motor_shaft_h - motor_flat_length) - hub_bottom_z + relief_margin; // 1.7
bore_engagement_h = hub_h - bore_relief_h;                                           // 5.3
plate_reach = norm([ear_pitch / 2 + ear_r + plate_clearance, ear_y]); // Ear tips from the shaft.

assert(part == "all" || part == "base" || part == "platter" || part == "assembly",
       "part must be all, base, platter or assembly");
assert(flat_fit > 0 && flat_fit < round_bore_d, "flat_fit must leave two flats");
assert(bore_relief_h >= 0, "Round relief cannot have a negative height");
assert(bore_engagement_h >= 3.0, "Double-D engagement must be at least 3 mm");
assert(hub_bottom_z > motor_boss_h, "The turning hub must clear the fixed boss");
assert(base_h - recess_h - can_h >= 2.0, "The can must hang clear of the table");
assert(tower_r - plate_reach >= 3.0, "The tower needs a wall around the ear recess");
assert(cable_slot_w > cable_cover_w, "The wire cover must pass the slot");
echo(bore_relief_h_mm = bore_relief_h, double_d_engagement_mm = bore_engagement_h,
     hub_to_boss_mm = hub_bottom_z - motor_boss_h,
     can_above_table_mm = base_h - recess_h - can_h,
     tower_wall_at_ears_mm = tower_r - plate_reach);

// A double-D is the intersection of a round hole and a parallel-sided strip. The round
// relief is at the TOP in print coordinates: the platter prints top face down, so the
// hub's free end, which meets the motor first, is the top of the print.
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

// The mounting plate seen from above: the can's top joined to the two ear lobes.
module plate_2d() {
    hull() {
        translate([0, ear_y]) circle(d = can_d);
        for (side = [-1, 1]) translate([side * ear_pitch / 2, ear_y]) circle(r = ear_r);
    }
}

// Printed bottom down. The motor drops in from the top: its can through the pocket, its
// ears into the recess (which keeps it from turning), its wire cover into the slot.
// The pocket is open underneath, so a finger can push the motor back out.
module base() {
    difference() {
        cylinder(r1 = foot_r, r2 = tower_r, h = base_h);
        translate([0, ear_y, -boolean_overlap])
            cylinder(d = can_d + 2 * can_clearance, h = base_h + 2 * boolean_overlap);
        translate([0, 0, base_h - recess_h])
            linear_extrude(height = recess_h + boolean_overlap)
                offset(r = plate_clearance) plate_2d();
        translate([-cable_slot_w / 2, -foot_r - 1, -boolean_overlap])
            cube([cable_slot_w, foot_r + 1 + ear_y, base_h + 2 * boolean_overlap]);
    }
}

module top_face_grooves() {
    for (r = centre_rings_r)
        translate([0, 0, -boolean_overlap])
            difference() {
                cylinder(r = r + groove_w / 2, h = groove_h + boolean_overlap);
                translate([0, 0, -boolean_overlap])
                    cylinder(r = r - groove_w / 2, h = groove_h + 3 * boolean_overlap);
            }
    for (angle = [0, 90])
        rotate([0, 0, angle])
            translate([-cross_l / 2, -groove_w / 2, -boolean_overlap])
                cube([cross_l, groove_w, groove_h + boolean_overlap]);
}

// Printed top face down, hub up: the same orientation as the pointer whose fit passed.
// The bore is blind: the disc above it is the stop for the shaft tip.
module platter() {
    difference() {
        union() {
            cylinder(d = platter_d, h = platter_h);
            translate([0, 0, platter_h - boolean_overlap]) {
                cylinder(d = hub_d, h = hub_h + boolean_overlap);
                cylinder(d1 = fillet_d, d2 = hub_d, h = fillet_h + boolean_overlap);
            }
        }
        translate([0, 0, platter_h]) double_d_bore(flat_fit, hub_h);
        top_face_grooves();
        translate([0, 0, -boolean_overlap])
            linear_extrude(height = platter_h + 2 * boolean_overlap)
                polygon([[platter_d / 2 + 1, -notch_w / 2], [platter_d / 2 - notch_depth, 0],
                         [platter_d / 2 + 1, notch_w / 2]]);
    }
}

// Preview only: a stand-in motor so the stack-up can be checked by eye.
module dummy_motor() {
    color([0.75, 0.75, 0.78]) {
        translate([0, ear_y, -plate_h - can_h]) cylinder(d = can_d, h = can_h);
        translate([0, 0, -plate_h]) linear_extrude(height = plate_h) plate_2d();
        translate([-cable_cover_w / 2, ear_y - can_d / 2 - 3, -plate_h - can_h + 2])
            cube([cable_cover_w, 6, can_h - 2]);
        cylinder(d = motor_boss_d, h = motor_boss_h);
        cylinder(d = motor_shaft_d, h = motor_shaft_h);
    }
}

// In use, the plate's top is z = 0 and sits recess_h - plate_h below the base's top.
module assembly() {
    translate([0, 0, -plate_h - (base_h - recess_h)]) color([0.55, 0.62, 0.70]) base();
    dummy_motor();
    translate([0, 0, platter_bottom_z + platter_h]) mirror([0, 0, 1])
        color([0.90, 0.85, 0.72]) platter();
}

if (part == "base") base();
else if (part == "platter") platter();
else if (part == "assembly") assembly();
else {
    base();
    translate([foot_r + layout_gap + platter_d / 2, 0, 0]) platter();
}
