import { palette } from "./palette.js";

// ============================================================
// Page 33: Custom Font Test
// Tests: layout for an unbundled font specification (Text, Ul, Ol, mixed).
// ============================================================
export const page33CustomFontXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 33: Custom Font Test</Text>
  <!-- Text with non-bundled font (Arial) -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
    <Text fontSize="14" bold="true">Text with Arial (non-bundled font):</Text>
    <HStack gap="24" alignItems="end">
      <Text fontSize="14" fontFamily="Arial">Arial 14px</Text>
      <Text fontSize="18" fontFamily="Arial">Arial 18px</Text>
      <Text fontSize="24" fontFamily="Arial">Arial 24px</Text>
    </HStack>
    <Text fontSize="14" fontFamily="Arial">The quick brown fox jumps over the lazy dog. ABCDEFghijklmn 0123456789</Text>
  </VStack>
  <!-- Ul with custom font -->
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ul with Arial:</Text>
      <Ul fontSize="14" fontFamily="Arial">
        <Li>Bullet item A</Li>
        <Li>Bullet item B</Li>
        <Li>Bullet item C</Li>
      </Ul>
    </VStack>
    <!-- Ol with custom font -->
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ol with Arial:</Text>
      <Ol fontSize="14" fontFamily="Arial">
        <Li>Numbered item 1</Li>
        <Li>Numbered item 2</Li>
        <Li>Numbered item 3</Li>
      </Ol>
    </VStack>
  </HStack>
  <!-- Mixed fonts on same slide -->
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
    <Text fontSize="14" bold="true">Mixed fonts (Noto Sans JP + Arial):</Text>
    <HStack gap="24" alignItems="end">
      <Text fontSize="16" fontFamily="Noto Sans JP">Noto Sans JP: Hello World</Text>
      <Text fontSize="16" fontFamily="Arial">Arial: Hello World</Text>
    </HStack>
    <HStack gap="24" alignItems="end">
      <Text fontSize="16" fontFamily="Noto Sans JP">Noto Sans JP: こんにちは</Text>
      <Text fontSize="16" fontFamily="Arial">Arial: こんにちは</Text>
    </HStack>
  </VStack>
  <!-- Mixed fonts in lists -->
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ul with Noto Sans JP:</Text>
      <Ul fontSize="14" fontFamily="Noto Sans JP">
        <Li>日本語テキスト A</Li>
        <Li>日本語テキスト B</Li>
        <Li>日本語テキスト C</Li>
      </Ul>
    </VStack>
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ul with Arial:</Text>
      <Ul fontSize="14" fontFamily="Arial">
        <Li>English text A</Li>
        <Li>English text B</Li>
        <Li>English text C</Li>
      </Ul>
    </VStack>
  </HStack>
</VStack>
`;

// ============================================================
// Page 34: Custom Font Word Wrap Test
// Tests: wrap and height computation for unbundled fonts.
// ============================================================
export const page34CustomFontWrapXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 34: Custom Font Word Wrap Test</Text>
  <!-- Text word wrap comparison: Noto Sans JP vs Arial -->
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Noto Sans JP (bundled) - narrow box:</Text>
      <Text w="200" backgroundColor="${palette.lightBlue}" padding="8" fontSize="14" fontFamily="Noto Sans JP">The quick brown fox jumps over the lazy dog and keeps running.</Text>
    </VStack>
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Arial (non-bundled) - narrow box:</Text>
      <Text w="200" backgroundColor="${palette.lightBlue}" padding="8" fontSize="14" fontFamily="Arial">The quick brown fox jumps over the lazy dog and keeps running.</Text>
    </VStack>
  </HStack>
  <!-- Ul word wrap comparison -->
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ul Noto Sans JP - long items:</Text>
      <Ul w="280" fontSize="14" fontFamily="Noto Sans JP">
        <Li>This is a long bullet item that should wrap to multiple lines in a narrow container</Li>
        <Li>Another long item to verify line height calculation with custom font settings</Li>
      </Ul>
    </VStack>
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ul Arial - long items:</Text>
      <Ul w="280" fontSize="14" fontFamily="Arial">
        <Li>This is a long bullet item that should wrap to multiple lines in a narrow container</Li>
        <Li>Another long item to verify line height calculation with custom font settings</Li>
      </Ul>
    </VStack>
  </HStack>
  <!-- Ol word wrap comparison -->
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ol Noto Sans JP - long items:</Text>
      <Ol w="280" fontSize="14" fontFamily="Noto Sans JP">
        <Li>First ordered item with enough text to trigger word wrapping in this container</Li>
        <Li>Second ordered item also long enough to verify correct height calculation</Li>
      </Ol>
    </VStack>
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Ol Arial - long items:</Text>
      <Ol w="280" fontSize="14" fontFamily="Arial">
        <Li>First ordered item with enough text to trigger word wrapping in this container</Li>
        <Li>Second ordered item also long enough to verify correct height calculation</Li>
      </Ol>
    </VStack>
  </HStack>
  <!-- CJK text wrap comparison -->
  <HStack gap="16" alignItems="stretch">
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Noto Sans JP - CJK narrow:</Text>
      <Text w="200" backgroundColor="${palette.lightBlue}" padding="8" fontSize="14" fontFamily="Noto Sans JP">日本語のテキストが狭いボックス内で正しく折り返されるか確認します。</Text>
    </VStack>
    <VStack w="50%" padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="8">
      <Text fontSize="14" bold="true">Arial - CJK narrow:</Text>
      <Text w="200" backgroundColor="${palette.lightBlue}" padding="8" fontSize="14" fontFamily="Arial">日本語のテキストが狭いボックス内で正しく折り返されるか確認します。</Text>
    </VStack>
  </HStack>
</VStack>
`;
