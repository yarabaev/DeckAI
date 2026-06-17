// Editable sample deck used by the preview pipeline (vrt/preview/runPreview.ts).
// Modify the XML here to verify a layout / rendering change end-to-end:
//   pnpm --filter @slideglance/builder run preview
// produces vrt/preview/output/sample.png for visual inspection.
//
// Also runnable standalone for raw PPTX output:
//   pnpm --filter @slideglance/builder exec tsx vrt/preview/sample.ts <out.pptx>

import path from "path";
import { fileURLToPath } from "url";

import { buildPptx } from "../../src/index.js";

const sampleXml = `
<VStack w="1280" h="720" padding.top="24" padding.bottom="24" padding.left="48" padding.right="48" gap="24" backgroundColor="F8FAFC">
  <Text fontSize="28" bold="true" color="1E293B">Icon Preset Library Demo</Text>

  <HStack gap="24" alignItems="center">
    <Icon name="cpu" size="48" color="#1D4ED8" />
    <Icon name="database" size="48" color="#16A34A" />
    <Icon name="cloud" size="48" color="#0EA5E9" />
    <Icon name="server" size="48" color="#DC2626" />
    <Icon name="shield" size="48" color="#0F172A" />
    <Icon name="star" size="48" color="#F59E0B" />
    <Icon name="heart" size="48" color="#DC2626" />
    <Icon name="zap" size="48" color="#F59E0B" />
    <Icon name="target" size="48" color="#16A34A" />
    <Icon name="lightbulb" size="48" color="#1D4ED8" />
  </HStack>

  <HStack gap="32" alignItems="end">
    <Icon name="briefcase" size="16" />
    <Icon name="briefcase" size="24" />
    <Icon name="briefcase" size="32" />
    <Icon name="briefcase" size="48" />
    <Icon name="briefcase" size="64" />
  </HStack>

  <HStack gap="24" alignItems="center">
    <Icon name="trending-up" size="32" color="#16A34A" />
    <Icon name="bar-chart" size="32" color="#1D4ED8" />
    <Icon name="pie-chart" size="32" color="#9333EA" />
    <Icon name="line-chart" size="32" color="#DC2626" />
  </HStack>
</VStack>
`;

export async function buildSamplePptx(outputPath: string): Promise<void> {
  const { pptx } = await buildPptx(sampleXml, { w: 1280, h: 720 });
  await pptx.writeFile({ fileName: outputPath });
}

// CLI entry — `tsx vrt/preview/sample.ts [outPath]`.
const invokedAsScript =
  process.argv[1] &&
  fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);
if (invokedAsScript) {
  const outputPath = process.argv[2] ?? "sample.pptx";
  buildSamplePptx(outputPath).catch((err) => {
    console.error(err instanceof Error ? err.message : String(err));
    process.exit(1);
  });
}
