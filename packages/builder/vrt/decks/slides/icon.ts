import { palette } from "./palette.js";

// ============================================================
// Page 25: Icon Test
// Tests: name, size, color, variant, bgColor.
// ============================================================
// ============================================================
// Page 29: Icon in HStack Test
// Tests: an icon directly under an HStack is not stretched.
// ============================================================
export const page29IconInHStackXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 29: Icon in HStack</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Icons directly in HStack (no w specified):</Text>
    <HStack gap="16" alignItems="center">
      <Icon name="cpu" size="32" color="#${palette.blue}" />
      <Icon name="database" size="32" color="#${palette.green}" />
      <Icon name="shield" size="32" color="#${palette.red}" />
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Icons with text siblings in HStack:</Text>
    <HStack gap="16" alignItems="center">
      <Icon name="star" size="32" color="#${palette.navy}" />
      <Text fontSize="16">Star icon with text</Text>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Variant icons in HStack:</Text>
    <HStack gap="16" alignItems="center">
      <Icon name="cpu" size="32" variant="circle-filled" backgroundColor="#E8F0FE" color="#${palette.blue}" />
      <Icon name="database" size="32" variant="circle-filled" backgroundColor="#E6F4EA" color="#${palette.green}" />
      <Icon name="shield" size="32" variant="circle-filled" backgroundColor="#FDE7E7" color="#${palette.red}" />
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Narrow HStack (w=120) with icons:</Text>
    <HStack w="120" gap="8" alignItems="center">
      <Icon name="cpu" size="32" color="#${palette.blue}" />
      <Icon name="database" size="32" color="#${palette.green}" />
      <Icon name="shield" size="32" color="#${palette.red}" />
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">HStack with default alignItems:</Text>
    <HStack gap="16">
      <Icon name="star" size="32" color="#${palette.navy}" />
      <Icon name="heart" size="32" color="#${palette.red}" />
      <Text fontSize="16">Text alongside icons</Text>
    </HStack>
  </VStack>
</VStack>
`;

// ============================================================
// Page 35: Svg Node Test
// Tests: Svg node combined with w/h/color.
// ============================================================
export const page35SvgNodeXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 35: Svg Node</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
      <Text fontSize="14" bold="true">Svg with size variations:</Text>
      <HStack gap="24" alignItems="center">
        <VStack gap="4" alignItems="center">
          <Svg>
            <svg viewBox="0 0 24 24"><polygon points="12,2 2,22 22,22" fill="none" stroke="black" stroke-width="2" stroke-linejoin="round"/></svg>
          </Svg>
          <Text fontSize="10">default</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Svg w="32" h="32">
            <svg viewBox="0 0 24 24"><polygon points="12,2 2,22 22,22" fill="none" stroke="black" stroke-width="2" stroke-linejoin="round"/></svg>
          </Svg>
          <Text fontSize="10">32x32</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Svg w="48" h="48">
            <svg viewBox="0 0 24 24"><polygon points="12,2 2,22 22,22" fill="none" stroke="black" stroke-width="2" stroke-linejoin="round"/></svg>
          </Svg>
          <Text fontSize="10">48x48</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Svg w="64" h="32">
            <svg viewBox="0 0 24 24"><polygon points="12,2 2,22 22,22" fill="none" stroke="black" stroke-width="2" stroke-linejoin="round"/></svg>
          </Svg>
          <Text fontSize="10">64x32</Text>
        </VStack>
      </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
      <Text fontSize="14" bold="true">Svg with color:</Text>
      <HStack gap="24" alignItems="center">
        <VStack gap="4" alignItems="center">
          <Svg w="32" h="32" color="#${palette.blue}">
            <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="none" stroke-width="2"/></svg>
          </Svg>
          <Text fontSize="10">blue</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Svg w="32" h="32" color="#${palette.red}">
            <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="none" stroke-width="2"/></svg>
          </Svg>
          <Text fontSize="10">red</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Svg w="32" h="32" color="#${palette.green}">
            <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="none" stroke-width="2"/></svg>
          </Svg>
          <Text fontSize="10">green</Text>
        </VStack>
      </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
      <Text fontSize="14" bold="true">Svg with fill/stroke preservation:</Text>
      <HStack gap="24" alignItems="center">
        <VStack gap="4" alignItems="center">
          <Svg w="32" h="32">
            <svg viewBox="0 0 24 24"><path d="M3 12l9-9 9 9" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><path d="M9 21V12h6v9" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </Svg>
          <Text fontSize="10">house</Text>
        </VStack>
        <VStack gap="4" alignItems="center">
          <Svg w="32" h="32">
            <svg viewBox="0 0 24 24"><path d="M12 2L2 7l10 5 10-5-10-5z" fill="#${palette.blue}" stroke="#${palette.navy}" stroke-width="1"/><path d="M2 17l10 5 10-5" fill="none" stroke="#${palette.navy}" stroke-width="1"/><path d="M2 12l10 5 10-5" fill="none" stroke="#${palette.navy}" stroke-width="1"/></svg>
          </Svg>
          <Text fontSize="10">layers</Text>
        </VStack>
      </HStack>
  </VStack>
</VStack>
`;

export const page25IconXml = `
<VStack w="100%" h="max" padding="48" gap="20" alignItems="stretch" backgroundColor="${palette.background}">
  <Text fontSize="28" color="${palette.charcoal}" bold="true">Page 25: Icon Test</Text>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Icon variations (default size 24px):</Text>
    <HStack gap="16" alignItems="center">
      <VStack gap="4" alignItems="center">
        <Icon name="cpu" />
        <Text fontSize="10">cpu</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="database" />
        <Text fontSize="10">database</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="cloud" />
        <Text fontSize="10">cloud</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="server" />
        <Text fontSize="10">server</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="globe" />
        <Text fontSize="10">globe</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="user" />
        <Text fontSize="10">user</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="mail" />
        <Text fontSize="10">mail</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="search" />
        <Text fontSize="10">search</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="settings" />
        <Text fontSize="10">settings</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="star" />
        <Text fontSize="10">star</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Size variations:</Text>
    <HStack gap="24" alignItems="end">
      <VStack gap="4" alignItems="center">
        <Icon name="heart" size="16" />
        <Text fontSize="10">16px</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="heart" size="24" />
        <Text fontSize="10">24px</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="heart" size="32" />
        <Text fontSize="10">32px</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="heart" size="48" />
        <Text fontSize="10">48px</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="heart" size="64" />
        <Text fontSize="10">64px</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Color variations:</Text>
    <HStack gap="24" alignItems="center">
      <VStack gap="4" alignItems="center">
        <Icon name="zap" size="32" color="#000000" />
        <Text fontSize="10">#000000</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="zap" size="32" color="#${palette.blue}" />
        <Text fontSize="10">blue</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="zap" size="32" color="#${palette.red}" />
        <Text fontSize="10">red</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="zap" size="32" color="#${palette.green}" />
        <Text fontSize="10">green</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="zap" size="32" color="#${palette.navy}" />
        <Text fontSize="10">navy</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Business icons:</Text>
    <HStack gap="16" alignItems="center">
      <VStack gap="4" alignItems="center">
        <Icon name="briefcase" size="32" color="#${palette.navy}" />
        <Text fontSize="10">briefcase</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="building" size="32" color="#${palette.navy}" />
        <Text fontSize="10">building</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="bar-chart" size="32" color="#${palette.navy}" />
        <Text fontSize="10">bar-chart</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="trending-up" size="32" color="#${palette.green}" />
        <Text fontSize="10">trending-up</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="shield" size="32" color="#${palette.blue}" />
        <Text fontSize="10">shield</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="target" size="32" color="#${palette.red}" />
        <Text fontSize="10">target</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="lightbulb" size="32" color="#${palette.navy}" />
        <Text fontSize="10">lightbulb</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Variant - circle-filled:</Text>
    <HStack gap="16" alignItems="center">
      <VStack gap="4" alignItems="center">
        <Icon name="cpu" size="32" variant="circle-filled" backgroundColor="#E8F0FE" color="#${palette.blue}" />
        <Text fontSize="10">cpu</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="database" size="32" variant="circle-filled" backgroundColor="#E6F4EA" color="#${palette.green}" />
        <Text fontSize="10">database</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="shield" size="32" variant="circle-filled" backgroundColor="#FDE7E7" color="#${palette.red}" />
        <Text fontSize="10">shield</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Variant - circle-outlined:</Text>
    <HStack gap="16" alignItems="center">
      <VStack gap="4" alignItems="center">
        <Icon name="star" size="32" variant="circle-outlined" backgroundColor="#${palette.blue}" color="#${palette.blue}" />
        <Text fontSize="10">star</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="heart" size="32" variant="circle-outlined" backgroundColor="#${palette.red}" color="#${palette.red}" />
        <Text fontSize="10">heart</Text>
      </VStack>
    </HStack>
  </VStack>
  <VStack padding="16" backgroundColor="FFFFFF" border.color="${palette.border}" border.width="1" gap="12">
    <Text fontSize="14" bold="true">Variant - square-filled / square-outlined:</Text>
    <HStack gap="16" alignItems="center">
      <VStack gap="4" alignItems="center">
        <Icon name="settings" size="32" variant="square-filled" backgroundColor="#E8F0FE" color="#${palette.blue}" />
        <Text fontSize="10">sq-filled</Text>
      </VStack>
      <VStack gap="4" alignItems="center">
        <Icon name="lock" size="32" variant="square-outlined" backgroundColor="#${palette.navy}" color="#${palette.navy}" />
        <Text fontSize="10">sq-outlined</Text>
      </VStack>
    </HStack>
  </VStack>
</VStack>
`;
