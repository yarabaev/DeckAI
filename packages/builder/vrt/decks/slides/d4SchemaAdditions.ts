import { palette } from "./palette.js";

// ============================================================
// Page 38: D4 Schema Additions
// Tests: textVAlign, rotate, chart showValue/barGrouping/valAxis,
//        Shape fill.transparency, isDecorative
// ============================================================
export const page38D4SchemaAdditionsXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 38: D4 Schema Additions</Text>
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
      <Text fontSize="14" bold="true">Shape textVAlign</Text>
      <HStack gap="12" alignItems="stretch">
        <VStack gap="4" alignItems="center">
          <Shape shapeType="rect" w="100" h="70" fill.color="${palette.lightBlue}" line.color="${palette.blue}" line.width="1" textVAlign="top" fontSize="12">top</Shape>
          <Text fontSize="10">top</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Shape shapeType="rect" w="100" h="70" fill.color="${palette.lightBlue}" line.color="${palette.blue}" line.width="1" textVAlign="middle" fontSize="12">middle</Shape>
          <Text fontSize="10">middle</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Shape shapeType="rect" w="100" h="70" fill.color="${palette.lightBlue}" line.color="${palette.blue}" line.width="1" textVAlign="bottom" fontSize="12">bottom</Shape>
          <Text fontSize="10">bottom</Text>
        </VStack>
      </HStack>
    </VStack>
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
      <Text fontSize="14" bold="true">Shape fill.transparency</Text>
      <HStack gap="12" alignItems="stretch">
        <VStack gap="4" alignItems="center">
          <Shape shapeType="rect" w="80" h="60" fill.color="${palette.blue}" fill.transparency="0" fontSize="10" color="FFFFFF">0%</Shape>
          <Text fontSize="10">opacity 100%</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Shape shapeType="rect" w="80" h="60" fill.color="${palette.blue}" fill.transparency="0.5" fontSize="10" color="FFFFFF">50%</Shape>
          <Text fontSize="10">opacity 50%</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Shape shapeType="rect" w="80" h="60" fill.color="${palette.blue}" fill.transparency="0.8" fontSize="10" color="FFFFFF">80%</Shape>
          <Text fontSize="10">opacity 20%</Text>
        </VStack>
      </HStack>
    </VStack>
  </HStack>
  <VStack padding="16" backgroundColor="FFFFFF" border='{"color":"${palette.border}","width":1}' gap="12">
    <Text fontSize="14" bold="true">Chart: showValue + barGrouping + valAxis</Text>
    <HStack gap="16" alignItems="stretch">
      <VStack w="50%" gap="4">
        <Text fontSize="12">showValue=true, barGrouping=clustered</Text>
        <Chart chartType="bar" w="400" h="160" showValue="true" barGrouping="clustered" valAxisMinVal="0" valAxisMaxVal="300">
          <ChartSeries name="Sales">
            <ChartDataPoint label="Q1" value="100" />
            <ChartDataPoint label="Q2" value="200" />
            <ChartDataPoint label="Q3" value="250" />
          </ChartSeries>
        </Chart>
      </VStack>
      <VStack w="50%" gap="4">
        <Text fontSize="12">showValue=true, barGrouping=stacked</Text>
        <Chart chartType="bar" w="400" h="160" showValue="true" barGrouping="stacked">
          <ChartSeries name="A">
            <ChartDataPoint label="Q1" value="80" />
            <ChartDataPoint label="Q2" value="120" />
            <ChartDataPoint label="Q3" value="100" />
          </ChartSeries>
          <ChartSeries name="B">
            <ChartDataPoint label="Q1" value="40" />
            <ChartDataPoint label="Q2" value="60" />
            <ChartDataPoint label="Q3" value="80" />
          </ChartSeries>
        </Chart>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;
