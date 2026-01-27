#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32) || defined(_WIN64)
#include <windows.h>
#endif

typedef int32_t (*tc_ide_service_compile_files_fn)(intptr_t, size_t, const char *const *);
typedef int32_t (*tc_free_handle_fn)(intptr_t);

typedef const char *(*fn_json_h_s)(intptr_t, const char *);
typedef const char *(*fn_json_h)(intptr_t);
typedef const char *(*fn_json_s)(const char *);
typedef const char *(*fn_json_s_usize_usize)(const char *, size_t, size_t);
typedef const char *(*fn_json_h_s_s)(intptr_t, const char *, const char *);
typedef const char *(*fn_json_h_s_i32)(intptr_t, const char *, int32_t);
typedef const char *(*fn_json_h_s_s_i32)(intptr_t, const char *, const char *, int32_t);

typedef int32_t (*fn_error_h)(intptr_t);
typedef int32_t (*fn_error_h_s)(intptr_t, const char *);
typedef int32_t (*fn_error_h_s_s)(intptr_t, const char *, const char *);

uint32_t tc_guarded_ide_service_compile_files(
    void *func_ptr,
    intptr_t ide_service_handle,
    size_t file_count,
    const char *const *files,
    int32_t *out_code)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_code = ((tc_ide_service_compile_files_fn)func_ptr)(ide_service_handle, file_count, files);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    (void)func_ptr;
    (void)ide_service_handle;
    (void)file_count;
    (void)files;
    (void)out_code;
    return 0;
#endif
}

uint32_t tc_guarded_free_handle(
    void *func_ptr,
    intptr_t handle,
    int32_t *out_code)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_code = ((tc_free_handle_fn)func_ptr)(handle);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    (void)func_ptr;
    (void)handle;
    (void)out_code;
    return 0;
#endif
}

uint32_t tc_guarded_json_h_s(
    void *func_ptr,
    intptr_t handle,
    const char *arg,
    const char **out_ptr)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_ptr = ((fn_json_h_s)func_ptr)(handle, arg);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_ptr = ((fn_json_h_s)func_ptr)(handle, arg);
    return 0;
#endif
}

uint32_t tc_guarded_json_h(
    void *func_ptr,
    intptr_t handle,
    const char **out_ptr)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_ptr = ((fn_json_h)func_ptr)(handle);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_ptr = ((fn_json_h)func_ptr)(handle);
    return 0;
#endif
}

uint32_t tc_guarded_json_s(
    void *func_ptr,
    const char *arg,
    const char **out_ptr)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_ptr = ((fn_json_s)func_ptr)(arg);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_ptr = ((fn_json_s)func_ptr)(arg);
    return 0;
#endif
}

uint32_t tc_guarded_json_s_usize_usize(
    void *func_ptr,
    const char *arg,
    size_t a,
    size_t b,
    const char **out_ptr)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_ptr = ((fn_json_s_usize_usize)func_ptr)(arg, a, b);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_ptr = ((fn_json_s_usize_usize)func_ptr)(arg, a, b);
    return 0;
#endif
}

uint32_t tc_guarded_json_h_s_s(
    void *func_ptr,
    intptr_t handle,
    const char *a,
    const char *b,
    const char **out_ptr)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_ptr = ((fn_json_h_s_s)func_ptr)(handle, a, b);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_ptr = ((fn_json_h_s_s)func_ptr)(handle, a, b);
    return 0;
#endif
}

uint32_t tc_guarded_json_h_s_i32(
    void *func_ptr,
    intptr_t handle,
    const char *a,
    int32_t i32_arg,
    const char **out_ptr)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_ptr = ((fn_json_h_s_i32)func_ptr)(handle, a, i32_arg);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_ptr = ((fn_json_h_s_i32)func_ptr)(handle, a, i32_arg);
    return 0;
#endif
}

uint32_t tc_guarded_json_h_s_s_i32(
    void *func_ptr,
    intptr_t handle,
    const char *a,
    const char *b,
    int32_t i32_arg,
    const char **out_ptr)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_ptr = ((fn_json_h_s_s_i32)func_ptr)(handle, a, b, i32_arg);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_ptr = ((fn_json_h_s_s_i32)func_ptr)(handle, a, b, i32_arg);
    return 0;
#endif
}

uint32_t tc_guarded_error_h(
    void *func_ptr,
    intptr_t handle,
    int32_t *out_code)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_code = ((fn_error_h)func_ptr)(handle);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_code = ((fn_error_h)func_ptr)(handle);
    return 0;
#endif
}

uint32_t tc_guarded_error_h_s(
    void *func_ptr,
    intptr_t handle,
    const char *a,
    int32_t *out_code)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_code = ((fn_error_h_s)func_ptr)(handle, a);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_code = ((fn_error_h_s)func_ptr)(handle, a);
    return 0;
#endif
}

uint32_t tc_guarded_error_h_s_s(
    void *func_ptr,
    intptr_t handle,
    const char *a,
    const char *b,
    int32_t *out_code)
{
#if defined(_WIN32) || defined(_WIN64)
    __try
    {
        *out_code = ((fn_error_h_s_s)func_ptr)(handle, a, b);
        return 0;
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        return (uint32_t)GetExceptionCode();
    }
#else
    *out_code = ((fn_error_h_s_s)func_ptr)(handle, a, b);
    return 0;
#endif
}
