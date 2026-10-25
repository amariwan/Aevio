import { useCallback, useEffect, useMemo, useState } from "react";
import * as api from "@/lib/api";
import { describeError } from "@/lib/api";
import type { Category, Client, Label, Project, Task } from "@/lib/types";
import { useTauriEvent } from "./useTauriEvent";

export interface Catalog {
  categories: Category[];
  projects: Project[];
  clients: Client[];
  tasks: Task[];
  labels: Label[];
  categoryById: Map<string, Category>;
  projectById: Map<string, Project>;
  clientById: Map<string, Client>;
  taskById: Map<string, Task>;
  error: string | null;
  reload: () => Promise<void>;
}

/** Categories, projects, and clients: the names every entry view resolves. */
export function useCatalog(): Catalog {
  const [categories, setCategories] = useState<Category[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [clients, setClients] = useState<Client[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [labels, setLabels] = useState<Label[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async (): Promise<void> => {
    try {
      const [nextCategories, nextProjects, nextClients, nextTasks, nextLabels] = await Promise.all([
        api.listCategories(),
        api.listProjects(),
        api.listClients(),
        api.listTasks(),
        api.listLabels(),
      ]);
      setCategories(nextCategories);
      setProjects(nextProjects);
      setClients(nextClients);
      setTasks(nextTasks);
      setLabels(nextLabels);
      setError(null);
    } catch (cause) {
      setError(describeError(cause));
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  useTauriEvent(api.ENTRIES_CHANGED, () => void reload());

  const categoryById = useMemo(
    () => new Map(categories.map((category) => [category.id, category])),
    [categories],
  );
  const projectById = useMemo(
    () => new Map(projects.map((project) => [project.id, project])),
    [projects],
  );
  const clientById = useMemo(
    () => new Map(clients.map((client) => [client.id, client])),
    [clients],
  );
  const taskById = useMemo(() => new Map(tasks.map((task) => [task.id, task])), [tasks]);

  return {
    categories,
    projects,
    clients,
    tasks,
    labels,
    categoryById,
    projectById,
    clientById,
    taskById,
    error,
    reload,
  };
}
