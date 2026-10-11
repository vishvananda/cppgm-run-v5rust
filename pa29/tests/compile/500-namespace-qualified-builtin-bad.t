// Only the global qualification names the builtin; a deeper one names
// something that does not exist.
namespace host { }

double wrong(double x) { return host::__builtin_fabs(x); }

int main() { return 0; }
