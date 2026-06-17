import { palette } from "./palette.js";

// ============================================================
// Page 18: Opacity Test
// Tests: opacity (background color transparency).
// ============================================================
export const page18OpacityXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 18: Opacity Test</Text>
  <!-- opacity variations -->
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="8">
    <Text fontSize="14" bold="true">opacity:</Text>
    <HStack gap="16" alignItems="stretch">
      <Text w="150" h="80" backgroundColor="${palette.blue}" opacity="1.0" fontSize="12" color="FFFFFF">opacity: 1.0</Text>
      <Text w="150" h="80" backgroundColor="${palette.blue}" opacity="0.8" fontSize="12" color="FFFFFF">opacity: 0.8</Text>
      <Text w="150" h="80" backgroundColor="${palette.blue}" opacity="0.5" fontSize="12" color="FFFFFF">opacity: 0.5</Text>
      <Text w="150" h="80" backgroundColor="${palette.blue}" opacity="0.2" fontSize="12" color="FFFFFF">opacity: 0.2</Text>
    </HStack>
  </VStack>
  <!-- Layer + opacity overlay -->
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="8">
    <Text fontSize="14" bold="true">Layer + opacity overlay:</Text>
    <Layer w="400" h="120">
      <Text x="0" y="0" w="400" h="120" backgroundColor="${palette.navy}" fontSize="16" color="FFFFFF">Background</Text>
      <Text x="0" y="0" w="400" h="120" backgroundColor="${palette.red}" opacity="0.4" fontSize="14" color="FFFFFF">Overlay (opacity: 0.4)</Text>
    </Layer>
  </VStack>
  <!-- Different node types with opacity -->
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="8">
    <Text fontSize="14" bold="true">Different nodes with opacity:</Text>
    <HStack gap="16" alignItems="stretch">
      <Text fontSize="14" backgroundColor="${palette.green}" opacity="0.5" color="FFFFFF" w="180" h="60">Text with opacity</Text>
      <VStack w="180" h="60" backgroundColor="${palette.accent}" opacity="0.5">
        <Text fontSize="14" color="FFFFFF">VStack with opacity</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;
