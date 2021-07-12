import { danger, warn, fail, markdown } from "danger";
import _ from "lodash";

type LabelRule = {
  path: string;
  label: string;
};

// ----------------------------------------------------------------------------
// CONFIG
const TRUSTED_USERS = ["sminez"];
const BIG_PR = 1000;

// ----------------------------------------------------------------------------
// DETAILS
const PR = danger.github.pr;
const GH_API = danger.github.api;
const MODIFIED = danger.git.modified_files;
const CREATED = danger.git.created_files;
const DELETED = danger.git.deleted_files;

console.log(`MODIFIED: ${MODIFIED}`);
console.log(`CREATED: ${CREATED}`);
console.log(`DELETED: ${DELETED}`);

// ----------------------------------------------------------------------------
// HELPERS
const trusted_users_only = (paths: string[]) => {
  paths.map((path: string) => {
    if (MODIFIED.includes(path) && !TRUSTED_USERS.includes(PR.user.login)) {
      fail(`:no_entry: Please do not modify ${path}`);
    }
  });
};

const modifies_dir = (dir: string): boolean => {
  return MODIFIED.filter((path: string) => path.startsWith(dir)).length > 0;
};

const update_labels = async (initial_labels: string[], rules: LabelRule[]) => {
  const { labels, new_labels } = rules.reduce(
    (acc, r) => {
      if (modifies_dir(r.path) && !acc.labels.includes(r.label)) {
        return { new_labels: true, labels: [...acc.labels, r.label] };
      }
      return acc;
    },
    { new_labels: false, labels: initial_labels },
  );

  if (new_labels) {
    await GH_API.issues.addLabels({
      issue_number: PR.number,
      owner: danger.github.thisPR.owner,
      repo: danger.github.thisPR.repo,
      labels: labels,
    });
  }
};

// ----------------------------------------------------------------------------
// RULES

// All PRs require a description
if (!PR.body || PR.body.length === 0) {
  fail(":memo: Please add a description to your PR summarising the change");
}

// Ensure that only trused users modify the following files
trusted_users_only(["Cargo.toml", "LICENSE"]);

// Highlight large PRs and request that they be broken down if possible
if (PR.additions + PR.deletions > BIG_PR) {
  warn(":exclamation: This looks like a big PR");
  markdown(
    "> The size of this PR seems relatively large. " +
      "If this PR contains multiple changes, spliting into " +
      "separate PRs helps with faster, easier review.",
  );
}

// Highlight newly added files
if (CREATED.length > 0) {
  const file_list = CREATED.join("\n");
  markdown(`:memo: This PR will add the following files:\n${file_list}`);
}

// Highlight deleted files
if (DELETED.length > 0) {
  const file_list = DELETED.join("\n");
  markdown(`:wastebasket: This PR deletes the following files:\n${file_list}`);
}

// Add labels based on modified files in the diff
update_labels(
  danger.github.issue.labels.map((label: { name: string }) => label.name),
  [
    { path: "crates", label: "crates" },
    { path: "src/core", label: "core" },
    { path: "src/draw", label: "draw" },
    { path: "src/contrib", label: "contrib" },
    { path: "src/x11rb", label: "x11rb" },
    { path: "src/xcb", label: "xcb" },
  ],
);

if (PR.author_association === "FIRST_TIME_CONTRIBUTOR") {
  markdown(":tada: Thank you for raising your first PR for penrose!\n!");
}

if (PR.body.match(/\.gif/g)) {
  markdown(":tophat: Oooooh! A GIF...nice\n!");
}
