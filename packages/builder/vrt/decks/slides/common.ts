import { palette } from "./palette.js";

// ============================================================
// Page 8: Common Properties Test
// Tests: w/h, padding, backgroundColor, border.
// ============================================================
export const page8CommonXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 8: Common Properties Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">w/h variations:</Text>
    <HStack gap="16" alignItems="end">
      <VStack gap="4" alignItems="center">
        <Text w="80" h="40" backgroundColor="${palette.blue}" text=""></Text>
        <Text fontSize="12">w:80, h:40</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Text w="30%" h="40" backgroundColor="${palette.blue}" text=""></Text>
        <Text fontSize="12">w:"30%"</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">padding variations:</Text>
    <HStack gap="16" alignItems="start">
      <VStack gap="4" alignItems="center">
        <VStack padding="8" backgroundColor="${palette.lightBlue}">
          <Text w="60" h="30" backgroundColor="${palette.blue}" text=""></Text>
        </VStack>
        <Text fontSize="12">padding: 8</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <VStack padding="24" backgroundColor="${palette.lightBlue}">
          <Text w="60" h="30" backgroundColor="${palette.blue}" text=""></Text>
        </VStack>
        <Text fontSize="12">padding: 24</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <VStack padding.top="20" padding.right="8" padding.bottom="4" padding.left="8" backgroundColor="${palette.lightBlue}">
          <Text w="60" h="30" backgroundColor="${palette.blue}" text=""></Text>
        </VStack>
        <Text fontSize="12">top:20, bottom:4</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">border variations:</Text>
    <HStack gap="16" alignItems="center">
      <VStack gap="4" alignItems="center">
        <Text w="80" h="40" border.color="${palette.charcoal}" border.width="1" text=""></Text>
        <Text fontSize="12">width: 1</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Text w="80" h="40" border.color="${palette.charcoal}" border.width="3" text=""></Text>
        <Text fontSize="12">width: 3</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Text w="80" h="40" border.color="${palette.blue}" border.width="2" text=""></Text>
        <Text fontSize="12">color: blue</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">backgroundColor variations:</Text>
    <HStack gap="16" alignItems="center">
      <Text w="80" h="40" backgroundColor="${palette.lightBlue}" text=""></Text>
      <Text w="80" h="40" backgroundColor="${palette.navy}" borderRadius="8" text=""></Text>
      <Text w="80" h="40" backgroundColor="${palette.blue}" borderRadius="16" text=""></Text>
      <Text w="80" h="40" backgroundColor="${palette.green}" borderRadius="20" text=""></Text>
    </HStack>
  </VStack>
</VStack>
`;
