// N3485 5.2.7p1-p2: dynamic_cast<T*>(v) requires T and *v to be complete class
// types (identity cast included). Must reject.
struct Inc;
int main() { Inc* p = 0; return dynamic_cast<const Inc*>(p) == 0; }
