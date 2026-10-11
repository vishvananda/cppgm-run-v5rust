#define CALL() 9
#define OPEN (
CALL OPEN )

#define RECUR(...) 1 RECUR(__VA_ARGS__)
#define IDENTITY(x) x
IDENTITY(RECUR(x))

#define f(x) x
#define g(x) x
g(f)(g)(3)
