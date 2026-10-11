// AUDIT-ID: ASSERT-MESSAGE
// AUDIT-EXPECT: reject
const char* operator "" _message(const char*, decltype(sizeof(0)));
static_assert(true, "text"_message);
