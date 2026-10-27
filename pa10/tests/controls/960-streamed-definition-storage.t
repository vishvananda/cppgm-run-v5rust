// expect exit 0
// Enough ordinary emitted definitions to exercise bounded text storage.
#define VALUE(n) int value_##n(int a) { int b = a + 3; int c = b * 5; return c - 7; }
#define ROW(n) VALUE(n##0) VALUE(n##1) VALUE(n##2) VALUE(n##3) VALUE(n##4) VALUE(n##5) VALUE(n##6) VALUE(n##7) VALUE(n##8) VALUE(n##9) VALUE(n##a) VALUE(n##b) VALUE(n##c) VALUE(n##d) VALUE(n##e) VALUE(n##f)
#define BLOCK(n) ROW(n##0) ROW(n##1) ROW(n##2) ROW(n##3) ROW(n##4) ROW(n##5) ROW(n##6) ROW(n##7) ROW(n##8) ROW(n##9) ROW(n##a) ROW(n##b) ROW(n##c) ROW(n##d) ROW(n##e) ROW(n##f)
BLOCK(0) BLOCK(1) BLOCK(2) BLOCK(3) BLOCK(4) BLOCK(5) BLOCK(6) BLOCK(7) BLOCK(8) BLOCK(9) BLOCK(a) BLOCK(b) BLOCK(c) BLOCK(d) BLOCK(e) BLOCK(f)
int main() { return value_abc(4) != 28; }
