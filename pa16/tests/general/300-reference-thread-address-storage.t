thread_local int thread_object;
constexpr int* thread_address(){return &thread_object;}
int*const& thread_reference=thread_address();
