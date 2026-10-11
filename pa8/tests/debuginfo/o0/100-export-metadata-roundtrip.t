declare global @__external_rtti__int : ptr [binding=strong, object=_ZTIi]

function @helper() -> i64 [binding=strong, object=_ZL6helperv, prefer_local=yes] {
  block ^entry:
    %0 = const i64 7 !dbg(sample.cpp, 1, 1)
    return i64 %0 !dbg(sample.cpp, 1, 1)
}

function @main() -> i64 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %0 = addr @__external_rtti__int !dbg(sample.cpp, 2, 1)
    %1 = call i64 @helper() !dbg(sample.cpp, 3, 1)
    return i64 %1 !dbg(sample.cpp, 3, 1)
}
