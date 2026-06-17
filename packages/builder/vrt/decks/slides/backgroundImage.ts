import { palette } from "./palette.js";

// ============================================================
// Page 20: Background Image Test
// Tests: backgroundImage (cover / contain).
// ============================================================
export const page20BackgroundImageXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 20: Background Image Test</Text>
  <!-- backgroundImage sizing modes -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="16" color="${palette.charcoal}" bold="true">Background Image Sizing Modes:</Text>
    <HStack gap="16">
      <!-- cover mode -->
      <Text w="280" h="180" backgroundImage.src="sample_images/sample_0.png" backgroundImage.sizing="cover" border.color="${palette.border}" border.width="2" fontSize="16" color="FFFFFF" bold="true">cover</Text>
      <!-- contain mode -->
      <Text w="280" h="180" backgroundImage.src="sample_images/sample_0.png" backgroundImage.sizing="contain" backgroundColor="333333" border.color="${palette.border}" border.width="2" fontSize="16" color="FFFFFF" bold="true">contain (with backgroundColor)</Text>
      <!-- default (cover) -->
      <Text w="280" h="180" backgroundImage.src="sample_images/sample_1.png" border.color="${palette.border}" border.width="2" fontSize="16" color="FFFFFF" bold="true">default (cover)</Text>
    </HStack>
  </VStack>
  <!-- VStack with backgroundImage -->
  <VStack gap="8" padding="16" backgroundImage.src="sample_images/sample_0.png" backgroundImage.sizing="cover" border.color="${palette.border}" border.width="1">
    <Text fontSize="16" color="FFFFFF" bold="true">VStack with backgroundImage</Text>
    <Text fontSize="14" color="FFFFFF">Background image on VStack container</Text>
  </VStack>
</VStack>
`;
