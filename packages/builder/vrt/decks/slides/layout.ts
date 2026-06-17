import { palette } from "./palette.js";

// ============================================================
// Page 7: Layout Test (VStack / HStack)
// Tests: gap, alignItems, justifyContent.
// ============================================================
export const page7LayoutXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 7: Layout Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">HStack gap:</Text>
    <HStack gap="8" alignItems="stretch">
      <HStack gap="8" padding="8" backgroundColor="${palette.lightBlue}">
        <Text w="40" h="30" backgroundColor="${palette.blue}" text=""></Text>
        <Text w="40" h="30" backgroundColor="${palette.blue}" text=""></Text>
        <Text w="40" h="30" backgroundColor="${palette.blue}" text=""></Text>
      </HStack>
      <Text fontSize="12">gap: 8</Text>
      <HStack gap="32" padding="8" backgroundColor="${palette.lightBlue}">
        <Text w="40" h="30" backgroundColor="${palette.blue}" text=""></Text>
        <Text w="40" h="30" backgroundColor="${palette.blue}" text=""></Text>
        <Text w="40" h="30" backgroundColor="${palette.blue}" text=""></Text>
      </HStack>
      <Text fontSize="12">gap: 32</Text>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">HStack alignItems:</Text>
    <HStack gap="16" alignItems="stretch">
      <VStack gap="4" alignItems="center">
        <HStack gap="4" alignItems="start" w="120" h="60" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" h="20" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" h="40" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">start</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack gap="4" alignItems="center" w="120" h="60" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" h="20" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" h="40" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">center</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack gap="4" alignItems="end" w="120" h="60" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" h="20" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" h="40" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">end</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack gap="4" alignItems="stretch" w="120" h="60" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">stretch</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">HStack justifyContent:</Text>
    <HStack gap="16" alignItems="stretch">
      <VStack gap="4" alignItems="center">
        <HStack gap="4" justifyContent="start" w="140" h="40" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">start</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack gap="4" justifyContent="center" w="140" h="40" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">center</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack gap="4" justifyContent="end" w="140" h="40" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">end</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack justifyContent="spaceBetween" w="140" h="40" padding="4" backgroundColor="${palette.lightBlue}">
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
          <Text w="30" h="30" backgroundColor="${palette.blue}" text=""></Text>
        </HStack>
        <Text fontSize="12">spaceBetween</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;

// ============================================================
// Page 16: Layer Node Test
// Tests: LayerNode — absolute placement, overlapping children, layer inside a VStack.
// ============================================================
export const page16LayerXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 16: Layer Node Test</Text>
  <!-- basic absolute placement (overlapping shapes) -->
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
      <Text fontSize="14" bold="true">Overlapping Shapes:</Text>
      <Layer w="500" h="200" backgroundColor="F0F4F8">
        <Shape shapeType="rect" w="120" h="100" x="30" y="30" fill.color="${palette.blue}" color="FFFFFF" fontSize="14">Back</Shape>
        <Shape shapeType="rect" w="120" h="100" x="80" y="60" fill.color="${palette.red}" color="FFFFFF" fontSize="14">Front</Shape>
        <Text x="220" y="80" fontSize="12" color="${palette.charcoal}">Shapes overlap (red is on top)</Text>
      </Layer>
    </VStack>
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
      <Text fontSize="14" bold="true">VStack inside Layer:</Text>
      <Layer w="500" h="200" backgroundColor="F0F4F8">
        <VStack x="20" y="20" w="180" gap="8" padding="12" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1">
          <Text fontSize="12" bold="true">Left VStack</Text>
          <Text fontSize="11">Item 1</Text>
          <Text fontSize="11">Item 2</Text>
        </VStack>
        <VStack x="260" y="20" w="180" gap="8" padding="12" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1">
          <Text fontSize="12" bold="true">Right VStack</Text>
          <Text fontSize="11">Item A</Text>
          <Text fontSize="11">Item B</Text>
        </VStack>
      </Layer>
    </VStack>
  </HStack>
  <!-- combined with a Line node -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Layer with Line (connection diagram):</Text>
    <Layer w="1100" h="200" backgroundColor="F8FAFC">
      <!-- left box -->
      <Shape shapeType="roundRect" w="150" h="80" x="50" y="60" fill.color="${palette.blue}" color="FFFFFF" fontSize="14">Service A</Shape>
      <!-- center box -->
      <Shape shapeType="roundRect" w="150" h="80" x="350" y="60" fill.color="${palette.green}" color="FFFFFF" fontSize="14">Service B</Shape>
      <!-- right box -->
      <Shape shapeType="roundRect" w="150" h="80" x="650" y="60" fill.color="${palette.accent}" color="FFFFFF" fontSize="14">Service C</Shape>
      <!-- connector line -->
      <Line x1="200" y1="100" x2="350" y2="100" color="333333" lineWidth="2" endArrow="true" />
      <Line x1="500" y1="100" x2="650" y2="100" color="333333" lineWidth="2" endArrow="true" />
      <!-- label -->
      <Text x="240" y="70" fontSize="10" color="${palette.charcoal}">API Call</Text>
      <Text x="550" y="70" fontSize="10" color="${palette.charcoal}">Event</Text>
    </Layer>
  </VStack>
  <!-- nested layer -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Nested Layer:</Text>
    <Layer w="600" h="150" backgroundColor="E3F2FD">
      <Text x="10" y="10" fontSize="12" bold="true">Outer Layer</Text>
      <Layer x="50" y="40" w="200" h="80" backgroundColor="FFF3E0">
        <Text x="10" y="30" fontSize="11">Inner Layer 1</Text>
      </Layer>
      <Layer x="280" y="40" w="200" h="80" backgroundColor="E8F5E9">
        <Text x="10" y="30" fontSize="11">Inner Layer 2</Text>
      </Layer>
    </Layer>
  </VStack>
</VStack>
`;

// ===================== Page 17: HStack + Table width computation =====================
export const page17HStackTableXml = `
<VStack gap="16" padding="48">
  <Text fontSize="22" bold="true">17. HStack + Table Width Calculation</Text>
  <!-- case where the table keeps its intrinsic size -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">HStack with table (table should keep intrinsic width):</Text>
    <HStack gap="16">
      <Text fontSize="14">Left text</Text>
      <Table defaultRowHeight="28">
        <Col w="80" />
        <Col w="120" />
        <Col w="80" />
        <Tr>
          <Td fontSize="12" bold="true" backgroundColor="${palette.lightBlue}">A</Td>
          <Td fontSize="12" bold="true" backgroundColor="${palette.lightBlue}">B</Td>
          <Td fontSize="12" bold="true" backgroundColor="${palette.lightBlue}">C</Td>
        </Tr>
        <Tr>
          <Td fontSize="12">1</Td>
          <Td fontSize="12">Data</Td>
          <Td fontSize="12">OK</Td>
        </Tr>
      </Table>
      <Text fontSize="14">Right text</Text>
    </HStack>
  </VStack>
</VStack>
`;
