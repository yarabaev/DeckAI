import { registerNode } from "./nodeRegistry.ts";
import { textNodeDef } from "./definitions/text.ts";
import { ulNodeDef, olNodeDef } from "./definitions/list.ts";
import { imageNodeDef } from "./definitions/image.ts";
import { tableNodeDef } from "./definitions/table.ts";
import { shapeNodeDef } from "./definitions/shape.ts";
import { chartNodeDef } from "./definitions/chart.ts";
import { iconNodeDef } from "./definitions/icon.ts";
import { lineNodeDef } from "./definitions/line.ts";
import { connectorNodeDef } from "./definitions/connector.ts";
import { vstackNodeDef, hstackNodeDef } from "./definitions/stack.ts";
import { layerNodeDef } from "./definitions/layer.ts";
import { svgNodeDef } from "./definitions/svg.ts";

// Register every node definition.
registerNode(textNodeDef);
registerNode(ulNodeDef);
registerNode(olNodeDef);
registerNode(imageNodeDef);
registerNode(tableNodeDef);
registerNode(shapeNodeDef);
registerNode(chartNodeDef);
registerNode(iconNodeDef);
registerNode(lineNodeDef);
registerNode(connectorNodeDef);
registerNode(vstackNodeDef);
registerNode(hstackNodeDef);
registerNode(layerNodeDef);
registerNode(svgNodeDef);

export { getNodeDef } from "./nodeRegistry.ts";
