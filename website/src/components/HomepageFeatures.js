import clsx from "clsx";
import React from "react";
import styles from "./HomepageFeatures.module.css";

const FeatureList = [
  {
    title: "Opinionated",
    Svg: require("@site/static/img/gavel-outline.svg").default,
    description: (
      <>
        Gofer follows
        <a href="https://12factor.net/"> cloud-native best practices</a> for
        configuring and running your short-lived jobs. It maintains simplicity
        by focusing on correct and maintainable container management. Avoiding
        the swiss army knife of mess that most CI/CD tools give and instead
        focuses on delivering an experience where you can be happy with strong,
        simple core functionality.
      </>
    ),
  },
  {
    title: "Pluggable",
    Svg: require("@site/static/img/plug-outline.svg").default,
    description: (
      <>
        Gofer provides pluggable interfaces to run on all your favorite
        cloud-native tooling. The default setup is easily run locally making it
        easy to develop against or troubleshoot. More advanced setups can
        leverage your favorite container orchestrator, object store, and more.
      </>
    ),
  },
  {
    title: "DAG(Directed Acyclic Graph) support",
    Svg: require("@site/static/img/project-diagram-outline.svg").default,
    description: (
      <>
        Run simple or complex graphs of containers to accomplish your tasks with
        full DAG support. It is possible to run containers in parallel, wait on
        other containers, or only run containers when dependent containers
        finish with a certain result.
      </>
    ),
  },
  {
    title: "Simplicity at its core",
    Svg: require("@site/static/img/th-large-outline.svg").default,
    description: (
      <>
        Detaching from pure gitops, and instead offering it as a feature allows
        users the ability to possess the same values (stability, predictability,
        reliability) that brings long-running jobs success. The ability to
        properly version, A/B test, and even canary out new versions of your
        short-lived jobs are all possible!
      </>
    ),
  },
];

function Feature({ Svg, title, description }) {
  return (
    <div className={clsx("col col--6")}>
      <div className="text--center">
        <Svg className={styles.featureSvg} alt={title} />
      </div>
      <div className="text--center padding-horiz--md">
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
