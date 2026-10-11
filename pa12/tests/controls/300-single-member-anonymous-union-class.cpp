// N3485 9.5p5: a class whose only member is an anonymous union with one
// member: k.u names the union member; expect exit 0
struct K { union { int u; }; };
int main() { K k; k.u = 1; return (k.u == 1 && sizeof(K) == sizeof(int)) ? 0 : 1; }
