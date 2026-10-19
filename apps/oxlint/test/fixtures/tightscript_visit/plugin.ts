import type { ESTree, Plugin } from "#oxlint/plugins";

type Node = ESTree.Node;

const plugin: Plugin = {
  meta: {
    name: "tightscript-visit",
  },
  rules: {
    "report-nodes": {
      create(context) {
        const reportVisit = (node: Node) =>
          context.report({ message: `Visited ${node.type}`, node });
        return {
          TSDistributeType: reportVisit,
          TSTypeReturn: reportVisit,
          TSProvideDeclaration: reportVisit,
          TSTypeFromDeclaration: reportVisit,
          TSTypeReference(node) {
            const name = node.typeName.type === "Identifier" ? node.typeName.name : "?";
            context.report({ message: `Reference ${name} in ${node.parent.type}`, node });
          },
        };
      },
    },
  },
};

export default plugin;
