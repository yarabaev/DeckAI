import { palette } from "./palette.js";

// ============================================================
// Page 27: HStack/VStack flexShrink Default Test
// Tests: %-size + gap stays inside the parent (flexShrink=1 by default).
// ============================================================
export const page27HStackFlexShrinkXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 27: flexShrink Default</Text>
  <!-- 25% x 4 + gap=12 must fit without overflow -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">25% x 4 + gap=12 (should fit without overflow):</Text>
    <HStack w="max" gap="12">
      <Text w="25%" h="60" backgroundColor="${palette.blue}" color="FFFFFF" fontSize="14">01</Text>
      <Text w="25%" h="60" backgroundColor="${palette.accent}" color="FFFFFF" fontSize="14">02</Text>
      <Text w="25%" h="60" backgroundColor="${palette.green}" color="FFFFFF" fontSize="14">03</Text>
      <Text w="25%" h="60" backgroundColor="${palette.red}" color="FFFFFF" fontSize="14">04</Text>
    </HStack>
  </VStack>
  <!-- 50% x 2 + gap=24 -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">50% x 2 + gap=24 (should fit without overflow):</Text>
    <HStack w="max" gap="24">
      <Text w="50%" h="60" backgroundColor="${palette.blue}" color="FFFFFF" fontSize="14">Left</Text>
      <Text w="50%" h="60" backgroundColor="${palette.accent}" color="FFFFFF" fontSize="14">Right</Text>
    </HStack>
  </VStack>
  <!-- 33% x 3 + gap=16 -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">33% x 3 + gap=16 (should fit without overflow):</Text>
    <HStack w="max" gap="16">
      <Text w="33%" h="60" backgroundColor="${palette.blue}" color="FFFFFF" fontSize="14">A</Text>
      <Text w="33%" h="60" backgroundColor="${palette.accent}" color="FFFFFF" fontSize="14">B</Text>
      <Text w="33%" h="60" backgroundColor="${palette.green}" color="FFFFFF" fontSize="14">C</Text>
    </HStack>
  </VStack>
  <!-- VStack: 50% x 2 + gap=16 (vertical) -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">VStack: 50% x 2 + gap=16 (should fit without overflow):</Text>
    <HStack gap="16" alignItems="stretch">
      <VStack w="50%" h="200" gap="16">
        <Text h="50%" backgroundColor="${palette.blue}" color="FFFFFF" fontSize="14">Top</Text>
        <Text h="50%" backgroundColor="${palette.accent}" color="FFFFFF" fontSize="14">Bottom</Text>
      </VStack>
      <VStack w="50%" h="200" gap="16">
        <Text h="33%" backgroundColor="${palette.green}" color="FFFFFF" fontSize="14">1</Text>
        <Text h="33%" backgroundColor="${palette.red}" color="FFFFFF" fontSize="14">2</Text>
        <Text h="33%" backgroundColor="${palette.blue}" color="FFFFFF" fontSize="14">3</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;
