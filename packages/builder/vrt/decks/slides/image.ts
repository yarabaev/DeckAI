import { palette } from "./palette.js";

// ============================================================
// Page 3: Image Test
// Tests: src, w, h.
// ============================================================
export const page3ImageXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 3: Image Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Size variations:</Text>
    <HStack gap="24" alignItems="end">
      <VStack gap="4" alignItems="center">
        <Image src="sample_images/sample_0.png" w="60" h="60" />
        <Text fontSize="12">60x60</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Image src="sample_images/sample_1.png" w="120" h="90" />
        <Text fontSize="12">120x90</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Image src="sample_images/sample_0.png" w="180" h="135" />
        <Text fontSize="12">180x135</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Image with container styling:</Text>
    <HStack gap="16" alignItems="start">
      <Image src="sample_images/sample_0.png" w="80" h="80" padding="12" backgroundColor="${palette.lightBlue}" border='{"color":"${palette.blue}","width":2}' />
      <VStack gap="4">
        <Text fontSize="16" bold="true">Image in styled VStack</Text>
        <Text fontSize="12">VStack with padding, backgroundColor, border</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;

// ============================================================
// Page 3b: Image Sizing Test
// Tests: sizing (contain, cover, crop).
// ============================================================
export const page3bImageSizingXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 3b: Image Sizing Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Sizing modes:</Text>
    <HStack gap="24" alignItems="end">
      <VStack gap="4" alignItems="center">
        <Image src="sample_images/sample_0.png" w="150" h="150" sizing='{"type":"contain"}' />
        <Text fontSize="12">contain</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Image src="sample_images/sample_0.png" w="150" h="150" sizing='{"type":"cover"}' />
        <Text fontSize="12">cover</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Image src="sample_images/sample_0.png" w="150" h="150" sizing='{"type":"crop","w":100,"h":100,"x":10,"y":10}' />
        <Text fontSize="12">crop (100x100)</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;
