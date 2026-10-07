-- The user role gains task_executions so users can see the tasks for runs they start.
UPDATE roles
SET permissions = '[{"resources":[{"namespaces":"^default$"},{"pipelines":".*"},"configs","deployments","events","objects","runs","secrets","subscriptions","task_executions"],"actions":["read","write","delete"]}]'
WHERE id = 'user' AND system_role = 1;

-- Each permission is now evaluated on its own, so extension roles need every resource a route requires inside a
-- single permission. Extensions also never had access to their own object store; this adds it.
--
-- Extension role ids are 'extension_<extension_id>'. User created role ids can't contain underscores so this can't
-- match them.
UPDATE roles
SET permissions = '[{"resources":[{"extensions":"^' || substr(id, 11) || '$"},"objects"],"actions":["read","write","delete"]},{"resources":[{"namespaces":".*"},{"pipelines":".*"},"runs"],"actions":["read","write"]},{"resources":["configs","deployments","events",{"namespaces":".*"},{"pipelines":".*"},"runs","subscriptions","system","task_executions"],"actions":["read"]}]'
WHERE id LIKE 'extension\_%' ESCAPE '\' AND system_role = 1;
