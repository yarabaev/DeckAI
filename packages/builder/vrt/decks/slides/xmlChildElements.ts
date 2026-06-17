import { palette } from "./palette.js";

// ============================================================
// Page 21: XML Child Element Notation Test
// Tests: parseXml child-element notation (Chart, Table, Ul/Ol).
// ============================================================
export const page21XmlChildElementsXml = `
<VStack gap="8" padding="48" backgroundColor="${palette.background}">
  <Text fontSize="28" bold="true" color="${palette.navy}">XML Child Element Notation</Text>
  <HStack gap="16" alignItems="start">
    <VStack gap="8" w="380">
      <Text fontSize="14" bold="true" color="${palette.charcoal}">Chart (Series/DataPoint)</Text>
      <Chart chartType="bar" w="380" h="140">
        <ChartSeries name="Q1">
          <ChartDataPoint label="Jan" value="100" />
          <ChartDataPoint label="Feb" value="120" />
          <ChartDataPoint label="Mar" value="90" />
        </ChartSeries>
      </Chart>
      <Text fontSize="14" bold="true" color="${palette.charcoal}">Table (Column/Row/Cell)</Text>
      <Table w="380">
        <Col w="190" />
        <Col w="190" />
        <Tr>
          <Td bold="true" backgroundColor="${palette.lightBlue}">Name</Td>
          <Td bold="true" backgroundColor="${palette.lightBlue}">Score</Td>
        </Tr>
        <Tr>
          <Td>Alice</Td>
          <Td>95</Td>
        </Tr>
        <Tr>
          <Td>Bob</Td>
          <Td>87</Td>
        </Tr>
      </Table>
    </VStack>
    <VStack gap="8" w="380">
      <Text fontSize="14" bold="true" color="${palette.charcoal}">Ul (Li children)</Text>
      <Ul fontSize="14">
        <Li>Plan the spec</Li>
        <Li>Implement the feature</Li>
        <Li>Ship behind a flag</Li>
      </Ul>
      <Text fontSize="14" bold="true" color="${palette.charcoal}">Ol (Li children + numberType)</Text>
      <Ol fontSize="14" numberType="arabicPeriod">
        <Li>Receive request</Li>
        <Li>Validate payload</Li>
        <Li>Persist to store</Li>
      </Ol>
    </VStack>
  </HStack>
</VStack>
`;
