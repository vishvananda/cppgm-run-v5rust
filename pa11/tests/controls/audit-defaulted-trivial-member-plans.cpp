// expect exit 0
struct Leaf {};
struct Branch { Leaf leaf; Branch() = default; };
struct Root { Branch branches[2]; Root() = default; };
int main() {
    Root root;
    return sizeof(root) == 2 ? 0 : 1;
}
