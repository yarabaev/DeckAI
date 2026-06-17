import { palette } from "./palette.js";

// ============================================================
// Page 37: Table cellBorder Test
// Tests: cellBorder (color / width / dashType).
// ============================================================
export const page37TableCellBorderXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 37: Table cellBorder Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">cellBorder with default style:</Text>
    <Table defaultRowHeight="32" cellBorder='{"color":"${palette.navy}","width":1}'>
      <Col w="150" />
      <Col w="150" />
      <Col w="150" />
      <Tr>
        <Td fontSize="14" bold="true" backgroundColor="${palette.lightBlue}">Header 1</Td>
        <Td fontSize="14" bold="true" backgroundColor="${palette.lightBlue}">Header 2</Td>
        <Td fontSize="14" bold="true" backgroundColor="${palette.lightBlue}">Header 3</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">Cell A</Td>
        <Td fontSize="13">Cell B</Td>
        <Td fontSize="13">Cell C</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">Cell D</Td>
        <Td fontSize="13">Cell E</Td>
        <Td fontSize="13">Cell F</Td>
      </Tr>
    </Table>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">cellBorder with dashType:</Text>
    <Table defaultRowHeight="32" cellBorder='{"color":"${palette.blue}","width":2,"dashType":"dash"}'>
      <Col w="150" />
      <Col w="150" />
      <Tr>
        <Td fontSize="13">Dashed</Td>
        <Td fontSize="13">Border</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">Style</Td>
        <Td fontSize="13">Test</Td>
      </Tr>
    </Table>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Table without cellBorder (default = no border):</Text>
    <Table defaultRowHeight="32">
      <Col w="150" />
      <Col w="150" />
      <Tr>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}">No</Td>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}">Border</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">Default</Td>
        <Td fontSize="13">Behavior</Td>
      </Tr>
    </Table>
  </VStack>
</VStack>
`;
