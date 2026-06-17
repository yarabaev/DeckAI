import { palette } from "./palette.js";

// ============================================================
// Page 28: Layout V2 - margin, zIndex, position, alignSelf, flexWrap
// ============================================================
export const page28LayoutV2Xml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 28: Layout V2</Text>

  <!-- margin test -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">margin:</Text>
    <HStack gap="16" alignItems="start">
      <VStack gap="4" alignItems="center">
        <VStack w="200" h="80" backgroundColor="${palette.lightBlue}">
          <Text w="60" h="40" margin="16" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
        </VStack>
        <Text fontSize="11">margin="16"</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <VStack w="200" h="80" backgroundColor="${palette.lightBlue}">
          <Text w="60" h="40" margin.top="8" margin.left="24" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
        </VStack>
        <Text fontSize="11">margin.top="8" margin.left="24"</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack gap="0" backgroundColor="${palette.lightBlue}" w="200" h="80">
          <Text w="50" h="40" margin="8" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
          <Text w="50" h="40" margin="8" backgroundColor="${palette.red}" text="" fontSize="10"></Text>
        </HStack>
        <Text fontSize="11">margin between siblings</Text>
      </VStack>
    </HStack>
  </VStack>

  <!-- zIndex test -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">zIndex:</Text>
    <HStack gap="32" alignItems="start">
      <VStack gap="4" alignItems="center">
        <Layer w="200" h="120" backgroundColor="${palette.lightBlue}">
          <Shape shapeType="rect" w="80" h="60" x="20" y="20" fill.color="${palette.blue}" color="FFFFFF" fontSize="12" zIndex="2">z:2</Shape>
          <Shape shapeType="rect" w="80" h="60" x="60" y="40" fill.color="${palette.red}" color="FFFFFF" fontSize="12" zIndex="1">z:1</Shape>
        </Layer>
        <Text fontSize="11">Blue(z:2) over Red(z:1)</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Layer w="200" h="120" backgroundColor="${palette.lightBlue}">
          <Shape shapeType="rect" w="80" h="60" x="20" y="20" fill.color="${palette.blue}" color="FFFFFF" fontSize="12" zIndex="1">z:1</Shape>
          <Shape shapeType="rect" w="80" h="60" x="60" y="40" fill.color="${palette.red}" color="FFFFFF" fontSize="12" zIndex="2">z:2</Shape>
        </Layer>
        <Text fontSize="11">Red(z:2) over Blue(z:1)</Text>
      </VStack>
    </HStack>
  </VStack>

  <!-- position absolute test -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">position="absolute":</Text>
    <HStack gap="32" alignItems="start">
      <VStack gap="4" alignItems="center">
        <VStack w="200" h="120" padding="8" gap="8" backgroundColor="${palette.lightBlue}">
          <Text w="60" h="30" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
          <Text w="60" h="30" backgroundColor="${palette.green}" text="" fontSize="10"></Text>
          <Text w="40" h="40" position="absolute" top="10" right="10" backgroundColor="${palette.red}" text="" fontSize="10"></Text>
        </VStack>
        <Text fontSize="11">absolute top/right badge</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <VStack w="200" h="120" padding="8" gap="8" backgroundColor="${palette.lightBlue}">
          <Text w="60" h="30" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
          <Text w="180" h="30" position="absolute" bottom="0" left="0" backgroundColor="${palette.accent}" fontSize="10" color="FFFFFF" textAlign="center">overlay footer</Text>
        </VStack>
        <Text fontSize="11">absolute bottom overlay</Text>
      </VStack>
    </HStack>
  </VStack>

  <!-- alignSelf test -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">alignSelf:</Text>
    <VStack w="300" h="120" gap="4" padding="8" backgroundColor="${palette.lightBlue}" alignItems="start">
      <Text w="80" h="24" backgroundColor="${palette.blue}" alignSelf="start" fontSize="10" color="FFFFFF" textAlign="center">start</Text>
      <Text w="80" h="24" backgroundColor="${palette.green}" alignSelf="center" fontSize="10" color="FFFFFF" textAlign="center">center</Text>
      <Text w="80" h="24" backgroundColor="${palette.red}" alignSelf="end" fontSize="10" color="FFFFFF" textAlign="center">end</Text>
      <Text h="24" backgroundColor="${palette.accent}" alignSelf="stretch" fontSize="10" color="FFFFFF" textAlign="center">stretch</Text>
    </VStack>
  </VStack>

  <!-- flexWrap test -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">flexWrap:</Text>
    <HStack gap="16" alignItems="start">
      <VStack gap="4" alignItems="center">
        <HStack w="200" gap="8" padding="8" backgroundColor="${palette.lightBlue}" flexWrap="wrap">
          <Text w="60" h="30" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
          <Text w="60" h="30" backgroundColor="${palette.green}" text="" fontSize="10"></Text>
          <Text w="60" h="30" backgroundColor="${palette.red}" text="" fontSize="10"></Text>
          <Text w="60" h="30" backgroundColor="${palette.accent}" text="" fontSize="10"></Text>
        </HStack>
        <Text fontSize="11">wrap</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <HStack w="200" gap="8" padding="8" backgroundColor="${palette.lightBlue}" flexWrap="nowrap">
          <Text w="60" h="30" backgroundColor="${palette.blue}" text="" fontSize="10"></Text>
          <Text w="60" h="30" backgroundColor="${palette.green}" text="" fontSize="10"></Text>
          <Text w="60" h="30" backgroundColor="${palette.red}" text="" fontSize="10"></Text>
          <Text w="60" h="30" backgroundColor="${palette.accent}" text="" fontSize="10"></Text>
        </HStack>
        <Text fontSize="11">nowrap (default)</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;
