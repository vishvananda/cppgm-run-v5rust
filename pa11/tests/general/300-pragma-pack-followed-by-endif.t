#ifdef __cplusplus
#pragma pack(push, 1)
#endif
struct X { char c; int i; };
#pragma pack(pop)
int main() { return sizeof(X) != 5; }
