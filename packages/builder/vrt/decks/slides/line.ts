import { palette } from "./palette.js";

// ============================================================
// Page 15: Line Node Test
// Tests: LineNode — x1, y1, x2, y2, color, lineWidth, dashType, beginArrow, endArrow.
// ============================================================
export const page15LineXml = `
<VStack w="100%" h="max" padding="48" gap="16" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 15: Line Node Test</Text>
  <!-- horizontal / vertical / diagonal lines -->
  <Line x1="100" y1="100" x2="300" y2="100" color="FF0000" lineWidth="2" />
  <Line x1="100" y1="120" x2="100" y2="250" color="00FF00" lineWidth="2" />
  <Line x1="150" y1="120" x2="300" y2="250" color="0000FF" lineWidth="2" />
  <Line x1="350" y1="120" x2="200" y2="250" color="FF00FF" lineWidth="2" />
  <!-- lines with arrows -->
  <Line x1="400" y1="100" x2="600" y2="100" color="333333" lineWidth="2" endArrow="true" />
  <Line x1="400" y1="130" x2="600" y2="130" color="333333" lineWidth="2" beginArrow="true" endArrow="true" />
  <!-- explicit arrow type -->
  <Line x1="400" y1="160" x2="600" y2="160" color="1D4ED8" lineWidth="2" endArrow.type="diamond" />
  <Line x1="400" y1="190" x2="600" y2="190" color="16A34A" lineWidth="2" endArrow.type="stealth" />
  <Line x1="400" y1="220" x2="600" y2="220" color="DC2626" lineWidth="2" endArrow.type="oval" />
  <!-- dashed lines -->
  <Line x1="650" y1="100" x2="850" y2="100" color="333333" lineWidth="2" dashType="dash" />
  <Line x1="650" y1="130" x2="850" y2="130" color="333333" lineWidth="2" dashType="dashDot" />
  <Line x1="650" y1="160" x2="850" y2="160" color="333333" lineWidth="2" dashType="lgDash" />
  <!-- thickness variations -->
  <Line x1="650" y1="200" x2="850" y2="200" color="0F172A" lineWidth="1" />
  <Line x1="650" y1="220" x2="850" y2="220" color="0F172A" lineWidth="3" />
  <Line x1="650" y1="245" x2="850" y2="245" color="0F172A" lineWidth="6" />
  <!-- coexistence of endArrow="true" + endArrow.type dot-notation -->
  <Line x1="650" y1="280" x2="850" y2="280" color="7C3AED" lineWidth="2" endArrow="true" endArrow.type="triangle" />
  <Line x1="650" y1="310" x2="850" y2="310" color="DB2777" lineWidth="2" beginArrow="true" beginArrow.type="diamond" />
  <!-- diagonal lines + arrows (four directions) -->
  <Line x1="900" y1="100" x2="1050" y2="200" color="${palette.blue}" lineWidth="2" endArrow="true" />
  <Line x1="1100" y1="100" x2="950" y2="200" color="${palette.red}" lineWidth="2" endArrow="true" />
  <Line x1="900" y1="250" x2="1050" y2="150" color="${palette.green}" lineWidth="2" endArrow="true" />
  <Line x1="1100" y1="250" x2="950" y2="150" color="FF6600" lineWidth="2" endArrow="true" />
</VStack>
`;
