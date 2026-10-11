// VALIDATION: with a pointer to a derived class as the argument, the
// overload taking a pointer to the base is a better match than the one
// taking a pointer to the const base: the two derived-to-base conversions
// differ only in qualification, so the less qualified target wins.

struct node_base
{
  node_base* right;
};

struct node : node_base
{
  int value;
};

static node* right_of(node_base* x) { return static_cast<node*>(x->right); }
static const node* right_of(const node_base* x) { return static_cast<const node*>(x->right); }

static int describe(node*) { return 1; }
static int describe(const node*) { return 2; }

static int describe_reference(node_base&) { return 3; }
static int describe_reference(const node_base&) { return 4; }

int main()
{
  node leaf;
  leaf.right = 0;
  leaf.value = 5;
  node root;
  root.right = &leaf;
  root.value = 7;
  node* found = right_of(&root);
  if (found != &leaf) return 1;
  if (describe(found) != 1) return 2;
  const node* frozen = found;
  if (describe(frozen) != 2) return 3;
  if (describe_reference(root) != 3) return 4;
  const node& frozen_root = root;
  if (describe_reference(frozen_root) != 4) return 5;
  return 0;
}
