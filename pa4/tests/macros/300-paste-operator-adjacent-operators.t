// N3485 16.3.3p1: ## shall not occur at the beginning or end of a replacement
// list; #define S(x,y) x ## ## y is a valid definition (full program: exit 0)
#define S(x,y) x ## ## y
int main() { return 0; } // 0
