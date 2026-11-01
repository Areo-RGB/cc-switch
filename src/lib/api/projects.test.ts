import { projectsApi } from "./projects";

describe("projectsApi", () => {
  it("returns project records in the browser mock", async () => {
    const projects = await projectsApi.list();

    expect(projects.length).toBeGreaterThan(0);
    expect(projects[0]).toMatchObject({
      name: expect.any(String),
      path: expect.any(String),
      kind: expect.any(String),
    });
  });
});
