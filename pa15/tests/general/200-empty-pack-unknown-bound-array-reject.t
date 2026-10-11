// Expansion to no initializer elements cannot complete an unknown bound.
template<int... I> void run() { int values[] = { I... }; (void)values; }
int main() { run<>(); return 0; }
