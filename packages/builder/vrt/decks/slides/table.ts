import { palette } from "./palette.js";

// ============================================================
// Page 4: Table Test
// Tests: columns, rows, defaultRowHeight, cell properties.
// ============================================================
export const page4TableXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 4: Table Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Basic table (header + data rows):</Text>
    <Table defaultRowHeight="32">
      <Col w="100" />
      <Col w="200" />
      <Col w="100" />
      <Tr>
        <Td fontSize="14" bold="true" backgroundColor="${palette.lightBlue}">ID</Td>
        <Td fontSize="14" bold="true" backgroundColor="${palette.lightBlue}">Name</Td>
        <Td fontSize="14" bold="true" backgroundColor="${palette.lightBlue}">Status</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">001</Td>
        <Td fontSize="13">Item Alpha</Td>
        <Td fontSize="13">Active</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">002</Td>
        <Td fontSize="13">Item Beta</Td>
        <Td fontSize="13">Pending</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">003</Td>
        <Td fontSize="13">Item Gamma</Td>
        <Td fontSize="13">Done</Td>
      </Tr>
    </Table>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Cell textAlign (left / center / right):</Text>
    <Table defaultRowHeight="32">
      <Col w="150" />
      <Col w="150" />
      <Col w="150" />
      <Tr>
        <Td fontSize="13" textAlign="left" backgroundColor="${palette.lightBlue}">Left</Td>
        <Td fontSize="13" textAlign="center" backgroundColor="${palette.lightBlue}">Center</Td>
        <Td fontSize="13" textAlign="right" backgroundColor="${palette.lightBlue}">Right</Td>
      </Tr>
      <Tr>
        <Td fontSize="13" textAlign="left">Aligned left</Td>
        <Td fontSize="13" textAlign="center">Aligned center</Td>
        <Td fontSize="13" textAlign="right">Aligned right</Td>
      </Tr>
    </Table>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Cell backgroundColor &amp; color:</Text>
    <Table defaultRowHeight="32">
      <Col w="150" />
      <Col w="150" />
      <Col w="150" />
      <Tr>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}">Light Blue BG</Td>
        <Td fontSize="13" backgroundColor="${palette.navy}" color="FFFFFF">Navy BG + White</Td>
        <Td fontSize="13" color="${palette.blue}">Blue text</Td>
      </Tr>
    </Table>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Column width omitted (auto equal split):</Text>
    <Table w="450" defaultRowHeight="32">
      <Col />
      <Col />
      <Col />
      <Tr>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}" bold="true">Col 1</Td>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}" bold="true">Col 2</Td>
        <Td fontSize="13" backgroundColor="${palette.lightBlue}" bold="true">Col 3</Td>
      </Tr>
      <Tr>
        <Td fontSize="13">150px each</Td>
        <Td fontSize="13">150px each</Td>
        <Td fontSize="13">150px each</Td>
      </Tr>
    </Table>
  </VStack>
</VStack>
`;
