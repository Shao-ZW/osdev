#ifndef __GBLIBC_STDINT_H_
#define __GBLIBC_STDINT_H_

#undef NULL
#ifdef __cplusplus
#define NULL (nullptr)
#else
#define NULL ((void*)0)
#endif

typedef __INT8_TYPE__ int8_t;
typedef __INT16_TYPE__ int16_t;
typedef __INT32_TYPE__ int32_t;
typedef __INT64_TYPE__ int64_t;

typedef __UINT8_TYPE__ uint8_t;
typedef __UINT16_TYPE__ uint16_t;
typedef __UINT32_TYPE__ uint32_t;
typedef __UINT64_TYPE__ uint64_t;

typedef __UINTPTR_TYPE__ uintptr_t;
typedef __INTPTR_TYPE__ intptr_t;

typedef __SIZE_TYPE__ size_t;
typedef int64_t ssize_t;

typedef uint64_t time_t;
typedef int64_t time_diff_t;

#endif
