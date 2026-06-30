/**
 * #779 – GraphQL query depth and complexity limiting
 *
 * Tests exercise the validation rules directly against the real schema.
 */
import { makeExecutableSchema } from "@graphql-tools/schema";
import { buildResolvers } from "./graphql";
import { readFileSync } from "fs";
import { join } from "path";
import depthLimit from "graphql-depth-limit";
import { createComplexityLimitRule } from "graphql-query-complexity";
import { parse, validate } from "graphql";

const db = {} as never;

const typeDefs = readFileSync(join(__dirname, "schema.graphql"), "utf-8");
const schema = makeExecutableSchema({ typeDefs, resolvers: buildResolvers(db) });

function check(query: string, maxDepth: number, maxComplexity: number) {
  const rules = [
    depthLimit(maxDepth),
    createComplexityLimitRule(maxComplexity),
  ];
  return validate(schema, parse(query), rules);
}

describe("#779 depth/complexity limiting", () => {
  it("accepts a shallow query within depth limit", () => {
    // depth = 2: query → healthCheck → status
    const errors = check(`{ healthCheck { status } }`, 7, 1000);
    expect(errors).toHaveLength(0);
  });

  it("rejects a query that exceeds the depth limit", () => {
    // depth = 4: query → attestations → edges → node → id  (within 7)
    // Setting maxDepth=3 to trigger rejection at depth 4
    const query = `{
      attestations {
        edges {
          node {
            id
          }
        }
      }
    }`;
    const errors = check(query, 3, 1000);
    expect(errors.length).toBeGreaterThan(0);
    expect(errors[0].message).toMatch(/exceeds maximum/i);
  });

  it("accepts the same query when depth limit is high enough", () => {
    const query = `{
      attestations {
        edges {
          node {
            id
          }
        }
      }
    }`;
    const errors = check(query, 7, 1000);
    expect(errors).toHaveLength(0);
  });

  it("rejects a query that exceeds the complexity limit", () => {
    // Setting maxComplexity=1 forces almost any non-trivial query to fail
    const errors = check(`{ issuers { items { address } } }`, 7, 1);
    expect(errors.length).toBeGreaterThan(0);
  });
});
