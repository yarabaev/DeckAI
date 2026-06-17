import { palette } from "./palette.js";

// ============================================================
// Page 32: Center-aligned VStack with HStack children
// Tests: text width inside an HStack under VStack alignItems="center" doesn't collapse.
// ============================================================
export const page32CenterAlignHStackXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 32: Center-Align HStack</Text>

  <!-- HStack inside a VStack with alignItems="center" -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">VStack alignItems="center" with HStack child:</Text>
    <VStack gap="14" alignItems="center">
      <HStack gap="10" alignItems="center">
        <Text w="42" h="42" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
        <Text fontSize="20" bold="true">Title text should not wrap</Text>
      </HStack>
      <Text fontSize="14">Description text below centered HStack</Text>
    </VStack>
  </VStack>

  <!-- VStack -> VStack alignItems="center" -> HStack nesting -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">VStack -> VStack center -> HStack nesting:</Text>
    <HStack gap="16" alignItems="stretch">
      <VStack w="50%" padding="16" backgroundColor="${palette.lightBlue}" gap="10" alignItems="center">
        <HStack gap="8" alignItems="center">
          <Text w="30" h="30" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
          <Text fontSize="16" bold="true">Left panel title</Text>
        </HStack>
        <Text fontSize="12">Left panel description text</Text>
      </VStack>
      <VStack w="50%" padding="16" backgroundColor="${palette.lightBlue}" gap="10" alignItems="center">
        <HStack gap="8" alignItems="center">
          <Text w="30" h="30" backgroundColor="${palette.green}" text="" fontSize="10"></Text>
          <Text fontSize="16" bold="true">Right panel title</Text>
        </HStack>
        <Text fontSize="12">Right panel description text</Text>
      </VStack>
    </HStack>
  </VStack>

  <!-- alignItems="start" / "end" must behave the same -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">VStack alignItems="start" and "end":</Text>
    <HStack gap="16" alignItems="stretch">
      <VStack w="50%" padding="16" backgroundColor="${palette.lightBlue}" gap="10" alignItems="start">
        <HStack gap="8" alignItems="center">
          <Text w="30" h="30" backgroundColor="${palette.red}" text="" fontSize="10"></Text>
          <Text fontSize="16" bold="true">alignItems="start"</Text>
        </HStack>
        <Text fontSize="12">Text under start-aligned HStack</Text>
      </VStack>
      <VStack w="50%" padding="16" backgroundColor="${palette.lightBlue}" gap="10" alignItems="end">
        <HStack gap="8" alignItems="center">
          <Text w="30" h="30" backgroundColor="${palette.accent}" text="" fontSize="10"></Text>
          <Text fontSize="16" bold="true">alignItems="end"</Text>
        </HStack>
        <Text fontSize="12">Text under end-aligned HStack</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;
