// N3485 7.1.6p2: redundant cv-qualifiers in a decl-specifier-seq (const const
// int) are prohibited (except via typedef/template argument). Must reject.
const const int x = 0;
int main() { return x; }
