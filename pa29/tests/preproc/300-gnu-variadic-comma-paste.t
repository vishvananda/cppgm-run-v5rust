#define CALL(...) target(1, ##__VA_ARGS__)

CALL()
CALL(alpha, beta)
