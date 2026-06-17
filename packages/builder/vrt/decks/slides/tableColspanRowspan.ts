import { palette } from "./palette.js";

// ============================================================
// Page 23: Table Colspan/Rowspan Test
// Tests: colspan, rowspan.
// ============================================================
export const page23TableColspanRowspanXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 23: Table Colspan/Rowspan Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">colspan (3-column merge header):</Text>
    <Table defaultRowHeight="32">
      <Col w="100" />
      <Col w="100" />
      <Col w="100" />
      <Tr>
        <Td fontSize="14" bold="true" backgroundColor="${palette.lightBlue}" colspan="3" textAlign="center">Merged Header</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">A</Td>
        <Td fontSize="13">B</Td>
        <Td fontSize="13">C</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">D</Td>
        <Td fontSize="13">E</Td>
        <Td fontSize="13">F</Td>
      </Tr>
    </Table>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">rowspan (2-row merge):</Text>
    <Table defaultRowHeight="32">
      <Col w="100" />
      <Col w="100" />
      <Col w="100" />
      <Tr>
        <Td fontSize="13" bold="true" backgroundColor="${palette.lightBlue}" rowspan="2">Row Merge</Td>
        <Td fontSize="13">A</Td>
        <Td fontSize="13">B</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">C</Td>
        <Td fontSize="13">D</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">E</Td>
        <Td fontSize="13">F</Td>
        <Td fontSize="13">G</Td>
      </Tr>
    </Table>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">colspan + rowspan combined:</Text>
    <Table defaultRowHeight="32">
      <Col w="100" />
      <Col w="100" />
      <Col w="100" />
      <Tr>
        <Td fontSize="14" bold="true" backgroundColor="${palette.navy}" color="FFFFFF" colspan="2" rowspan="2" textAlign="center">2x2 Merge</Td>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}">Top</Td>
      </Tr>
      <Tr>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}">Mid</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">X</Td>
        <Td fontSize="13">Y</Td>
        <Td fontSize="13">Z</Td>
      </Tr>
    </Table>
  </VStack>
</VStack>
`;
