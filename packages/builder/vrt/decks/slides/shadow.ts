import { palette } from "./palette.js";

// ============================================================
// Page 19: Shadow Test
// Tests: VStack shadow, Image shadow, Shape shadow.
// ============================================================
export const page19ShadowXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 19: Shadow Test</Text>
  <!-- VStack with outer shadow -->
  <HStack gap="24" alignItems="start">
    <VStack w="200" h="100" padding="16" backgroundColor="FFFFFF" borderRadius="8" shadow.type="outer" shadow.color="000000" shadow.blur="6" shadow.offset="3" shadow.angle="315" shadow.opacity="0.3">
      <Text fontSize="14" color="${palette.charcoal}">VStack: outer shadow</Text>
    </VStack>
    <!-- VStack with inner shadow -->
    <VStack w="200" h="100" padding="16" backgroundColor="FFFFFF" borderRadius="8" shadow.type="inner" shadow.color="000000" shadow.blur="4" shadow.offset="2" shadow.angle="315" shadow.opacity="0.2">
      <Text fontSize="14" color="${palette.charcoal}">VStack: inner shadow</Text>
    </VStack>
    <!-- VStack with shadow + border -->
    <VStack w="200" h="100" padding="16" backgroundColor="FFFFFF" border.color="${palette.blue}" border.width="2" borderRadius="8" shadow.type="outer" shadow.color="${palette.blue}" shadow.blur="8" shadow.offset="4" shadow.angle="315" shadow.opacity="0.4">
      <Text fontSize="14" color="${palette.charcoal}">VStack: shadow + border</Text>
    </VStack>
  </HStack>
  <!-- Shape with shadow (various shape types) -->
  <HStack gap="24" alignItems="start">
    <Shape shapeType="ellipse" w="150" h="100" fill.color="${palette.lightBlue}" shadow.type="outer" shadow.color="000000" shadow.blur="6" shadow.offset="3" shadow.angle="315" shadow.opacity="0.3" fontSize="12" color="${palette.charcoal}">Ellipse shadow</Shape>
    <Shape shapeType="roundRect" w="150" h="100" fill.color="${palette.lightBlue}" shadow.type="outer" shadow.color="${palette.navy}" shadow.blur="10" shadow.offset="5" shadow.angle="270" shadow.opacity="0.5" fontSize="12" color="${palette.charcoal}">RoundRect shadow</Shape>
  </HStack>
  <!-- Image with shadow -->
  <HStack gap="24" alignItems="start">
    <Image src="https://placehold.co/180x120/DBEAFE/1D4ED8?text=Shadow" w="180" h="120" shadow.type="outer" shadow.color="000000" shadow.blur="8" shadow.offset="4" shadow.angle="315" shadow.opacity="0.4" />
  </HStack>
  <!-- VStack with shadow only (no background, no border) -->
  <HStack gap="24" alignItems="start">
    <VStack w="200" h="80" padding="16" shadow.type="outer" shadow.color="000000" shadow.blur="6" shadow.offset="3" shadow.angle="315" shadow.opacity="0.3">
      <Text fontSize="14" color="${palette.charcoal}">Shadow only (no bg)</Text>
    </VStack>
  </HStack>
</VStack>
`;

// ============================================================
// Page 26: VStack/HStack Shadow Test
// Tests: VStack shadow, HStack shadow.
// ============================================================
export const page26VStackHStackShadowXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 26: VStack/HStack Shadow Test</Text>
  <!-- VStack with shadow -->
  <HStack gap="24" alignItems="start">
    <VStack w="200" padding="16" gap="8" backgroundColor="FFFFFF" borderRadius="8" shadow.type="outer" shadow.color="000000" shadow.blur="6" shadow.offset="3" shadow.angle="315" shadow.opacity="0.3">
      <Text fontSize="14" color="${palette.charcoal}">VStack: outer shadow</Text>
      <Text fontSize="12" color="${palette.charcoal}">Card content</Text>
    </VStack>
    <VStack w="200" padding="16" gap="8" backgroundColor="FFFFFF" borderRadius="8" shadow.type="inner" shadow.color="000000" shadow.blur="4" shadow.offset="2" shadow.angle="315" shadow.opacity="0.2">
      <Text fontSize="14" color="${palette.charcoal}">VStack: inner shadow</Text>
      <Text fontSize="12" color="${palette.charcoal}">Card content</Text>
    </VStack>
  </HStack>
  <!-- HStack with shadow -->
  <HStack gap="24" alignItems="start">
    <HStack w="250" padding="16" gap="8" backgroundColor="FFFFFF" borderRadius="8" shadow.type="outer" shadow.color="000000" shadow.blur="6" shadow.offset="3" shadow.angle="315" shadow.opacity="0.3" alignItems="center">
      <Text fontSize="14" color="${palette.charcoal}">HStack: outer shadow</Text>
    </HStack>
    <HStack w="250" padding="16" gap="8" backgroundColor="FFFFFF" border.color="${palette.blue}" border.width="2" borderRadius="8" shadow.type="outer" shadow.color="${palette.blue}" shadow.blur="8" shadow.offset="4" shadow.angle="315" shadow.opacity="0.4" alignItems="center">
      <Text fontSize="14" color="${palette.charcoal}">HStack: shadow + border</Text>
    </HStack>
  </HStack>
</VStack>
`;
