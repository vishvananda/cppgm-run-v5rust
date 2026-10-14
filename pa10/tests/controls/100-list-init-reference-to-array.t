// N3485 8.5.4p3 (reference case) + 8.5.3p5: const int (&e)[2]{1, 2} binds to a
// list-initialized array temporary; expect exit 0
int main() { const int (&e)[2]{1, 2}; return e[0] + e[1] == 3 ? 0 : 1; }
