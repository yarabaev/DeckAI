import { palette } from "./palette.js";

// ============================================================
// Page 30: Leaf Node Padding Test
// Tests: content rendering position when a leaf node has padding set.
// ============================================================
export const page30LeafPaddingXml = `
<VStack w="100%" h="max" padding="48" gap="16" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 30: Leaf Node Padding</Text>

  <HStack gap="16" alignItems="stretch">
    <!-- Text with padding -->
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Text padding=20</Text>
      <Text padding="20" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" fontSize="14">
        Padded text content
      </Text>
    </VStack>

    <!-- Ul with padding -->
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Ul padding=20</Text>
      <Ul padding="20" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" fontSize="14">
        <Li>Item A</Li>
        <Li>Item B</Li>
      </Ul>
    </VStack>

    <!-- Ol with padding -->
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Ol padding=20</Text>
      <Ol padding="20" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" fontSize="14">
        <Li>First</Li>
        <Li>Second</Li>
      </Ol>
    </VStack>
  </HStack>

  <HStack gap="16" alignItems="stretch">
    <!-- Image with padding -->
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Image padding=20</Text>
      <Image padding="20" w="200" h="120" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" src="https://placehold.co/160x80/png" />
    </VStack>

    <!-- Shape with padding -->
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Shape padding=20</Text>
      <Shape padding="20" w="200" h="120" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" shapeType="roundRect" fill.color="${palette.blue}" text="Shape" color="FFFFFF" fontSize="14" />
    </VStack>

    <!-- Icon with padding -->
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Icon padding=20</Text>
      <Icon padding="20" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" name="star" size="32" color="#${palette.blue}" />
    </VStack>
  </HStack>

  <HStack gap="16" alignItems="stretch">
    <!-- Chart with padding -->
    <VStack gap="4" w="50%">
      <Text fontSize="12" bold="true">Chart padding=20</Text>
      <Chart padding="20" w="350" h="180" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" chartType="bar">
        <ChartSeries name="Sales">
          <ChartDataPoint label="Q1" value="10" />
          <ChartDataPoint label="Q2" value="20" />
          <ChartDataPoint label="Q3" value="30" />
        </ChartSeries>
      </Chart>
    </VStack>

    <!-- Table with padding -->
    <VStack gap="4" w="50%">
      <Text fontSize="12" bold="true">Table padding=20</Text>
      <Table padding="20" w="350" h="180" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1">
        <Col w="150" />
        <Col w="150" />
        <Tr>
          <Td fontSize="14" bold="true">A1</Td>
          <Td fontSize="14" bold="true">B1</Td>
        </Tr>
        <Tr>
          <Td fontSize="14">A2</Td>
          <Td fontSize="14">B2</Td>
        </Tr>
      </Table>
    </VStack>
  </HStack>
</VStack>
`;

// ============================================================
// Page 31: Leaf Node Padding - Asymmetric Padding
// Tests: asymmetric padding on remaining leaf nodes.
// ============================================================
export const page31LeafPaddingCompositeXml = `
<VStack w="100%" h="max" padding="48" gap="16" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 31: Asymmetric Padding</Text>

  <!-- Asymmetric padding test -->
  <HStack gap="16" alignItems="stretch">
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Text asymmetric</Text>
      <Text padding='{"top":10,"right":40,"bottom":30,"left":20}' backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" fontSize="14">
        Asymmetric padding
      </Text>
    </VStack>

    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Icon variant + padding</Text>
      <Icon padding="20" variant="circle-filled" backgroundColor="#${palette.blue}" border.color="${palette.border}" border.width="1" name="star" size="24" color="#FFFFFF" />
    </VStack>

    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Shape asymmetric</Text>
      <Shape padding='{"top":10,"right":40,"bottom":30,"left":20}' w="200" h="120" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" shapeType="rect" fill.color="${palette.green}" text="Asym" color="FFFFFF" fontSize="14" />
    </VStack>
  </HStack>

  <!-- Icon aspect ratio test: non-square container -->
  <HStack gap="16" alignItems="stretch">
    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Icon wide container</Text>
      <Icon w="200" h="80" padding="10" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" name="star" size="32" color="#${palette.blue}" />
    </VStack>

    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Icon tall container</Text>
      <Icon w="80" h="200" padding="10" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" name="heart" size="32" color="#${palette.red}" />
    </VStack>

    <VStack gap="4" w="33%">
      <Text fontSize="12" bold="true">Icon asymmetric padding</Text>
      <Icon padding='{"top":10,"right":40,"bottom":30,"left":20}' backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" name="cpu" size="32" color="#${palette.green}" />
    </VStack>
  </HStack>
</VStack>
`;
