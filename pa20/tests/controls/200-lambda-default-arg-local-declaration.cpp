// N3485 8.3.6p7, 5.1.2: a captureless lambda in a default argument of a
// block-scope function declaration (naming no local); expect exit 0
int main() { extern int use(int = [] { return 3; }()); return use() - 3; }
int use(int v) { return v; }
