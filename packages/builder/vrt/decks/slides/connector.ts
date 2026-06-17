import { palette } from "./palette.js";

// ============================================================
// Connector Node Test
// Tests: ConnectorNode — kind (straight / elbow / curved), explicit
// fromSide/toSide, auto side selection from bbox geometry, and the
// arrow / dash style passthrough.
//
// Layout uses <Layer> so both shapes carry explicit x/y, which keeps
// the post-process side-pair math deterministic across snapshot runs.
// ============================================================
export const pageConnectorXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Connector Node Test</Text>

  <!-- Row 1: kind variants on a horizontal pair -->
  <Layer w="1100" h="160" backgroundColor="F8FAFC">
    <Shape id="r1a" shapeType="roundRect" x="40"  y="50" w="160" h="60" fill.color="${palette.blue}"  color="FFFFFF" fontSize="14">straight</Shape>
    <Shape id="r1b" shapeType="roundRect" x="280" y="50" w="160" h="60" fill.color="${palette.green}" color="FFFFFF" fontSize="14">end</Shape>
    <Connector from="r1a" to="r1b" kind="straight" fromSide="right" toSide="left" endArrow="true" color="333333" lineWidth="2"/>

    <Shape id="r1c" shapeType="roundRect" x="500" y="50" w="160" h="60" fill.color="${palette.blue}"  color="FFFFFF" fontSize="14">elbow</Shape>
    <Shape id="r1d" shapeType="roundRect" x="740" y="50" w="160" h="60" fill.color="${palette.green}" color="FFFFFF" fontSize="14">end</Shape>
    <Connector from="r1c" to="r1d" kind="elbow" fromSide="right" toSide="left" endArrow="true" color="333333" lineWidth="2"/>
  </Layer>

  <!-- Row 2: curved + perpendicular side pairs -->
  <Layer w="1100" h="180" backgroundColor="F8FAFC">
    <Shape id="r2a" shapeType="roundRect" x="40"  y="40" w="160" h="60" fill.color="${palette.accent}" color="FFFFFF" fontSize="14">curved</Shape>
    <Shape id="r2b" shapeType="roundRect" x="280" y="40" w="160" h="60" fill.color="${palette.red}"    color="FFFFFF" fontSize="14">end</Shape>
    <Connector from="r2a" to="r2b" kind="curved" fromSide="right" toSide="left" endArrow="true" color="333333" lineWidth="2"/>

    <!-- perpendicular sides -> bentConnector2 (L shape) -->
    <Shape id="r2c" shapeType="roundRect" x="500" y="20"  w="160" h="60" fill.color="${palette.blue}"  color="FFFFFF" fontSize="14">top</Shape>
    <Shape id="r2d" shapeType="roundRect" x="740" y="100" w="160" h="60" fill.color="${palette.green}" color="FFFFFF" fontSize="14">right</Shape>
    <Connector from="r2c" to="r2d" kind="elbow" fromSide="right" toSide="top" endArrow="true" color="333333" lineWidth="2"/>
  </Layer>

  <!-- Row 3: same-side -> bentConnector4 (U shape), and auto side selection -->
  <Layer w="1100" h="200" backgroundColor="F8FAFC">
    <Shape id="r3a" shapeType="roundRect" x="40"  y="40"  w="160" h="60" fill.color="${palette.blue}"  color="FFFFFF" fontSize="14">same right</Shape>
    <Shape id="r3b" shapeType="roundRect" x="280" y="120" w="160" h="60" fill.color="${palette.green}" color="FFFFFF" fontSize="14">end</Shape>
    <Connector from="r3a" to="r3b" kind="elbow" fromSide="right" toSide="right" endArrow="true" color="333333" lineWidth="2"/>

    <!-- auto-side: horizontal neighbours -> right / left implicitly -->
    <Shape id="r3c" shapeType="roundRect" x="540" y="80" w="160" h="60" fill.color="${palette.accent}" color="FFFFFF" fontSize="14">auto h</Shape>
    <Shape id="r3d" shapeType="roundRect" x="780" y="80" w="160" h="60" fill.color="${palette.red}"    color="FFFFFF" fontSize="14">end</Shape>
    <Connector from="r3c" to="r3d" kind="straight" endArrow="true" color="333333" lineWidth="2"/>
  </Layer>

  <!-- Row 4: auto-side vertical + arrow / dash styling -->
  <Layer w="1100" h="200" backgroundColor="F8FAFC">
    <Shape id="r4a" shapeType="roundRect" x="100" y="20"  w="160" h="60" fill.color="${palette.blue}"  color="FFFFFF" fontSize="14">auto v</Shape>
    <Shape id="r4b" shapeType="roundRect" x="100" y="120" w="160" h="60" fill.color="${palette.green}" color="FFFFFF" fontSize="14">end</Shape>
    <Connector from="r4a" to="r4b" kind="straight" endArrow="true" beginArrow="true" color="${palette.red}" lineWidth="2"/>

    <Shape id="r4c" shapeType="roundRect" x="540" y="20"  w="160" h="60" fill.color="${palette.blue}"  color="FFFFFF" fontSize="14">dashed elbow</Shape>
    <Shape id="r4d" shapeType="roundRect" x="780" y="120" w="160" h="60" fill.color="${palette.green}" color="FFFFFF" fontSize="14">end</Shape>
    <Connector from="r4c" to="r4d" kind="elbow" endArrow="true" dashType="dash" color="${palette.blue}" lineWidth="2"/>
  </Layer>
</VStack>
`;
