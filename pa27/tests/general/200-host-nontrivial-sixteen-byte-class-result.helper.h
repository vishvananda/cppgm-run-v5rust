namespace result_abi {
struct Value {
    long long first;
    long long second;
    Value(long long);
    ~Value();
};
Value host_make(long long);
Value student_make(long long);
int host_check_student();
}
