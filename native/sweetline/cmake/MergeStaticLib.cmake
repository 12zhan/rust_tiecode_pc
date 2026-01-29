function(merge_static_libs TARGET_NAME LIB_PATH)
    if(MSVC)
        add_custom_command(TARGET ${TARGET_NAME} POST_BUILD
            COMMAND lib.exe /OUT:$<TARGET_FILE:${TARGET_NAME}> $<TARGET_FILE:${TARGET_NAME}> ${LIB_PATH}
        )
    elseif(APPLE)
        # On macOS, use libtool
        add_custom_command(TARGET ${TARGET_NAME} POST_BUILD
            COMMAND libtool -static -o $<TARGET_FILE:${TARGET_NAME}> $<TARGET_FILE:${TARGET_NAME}> ${LIB_PATH}
        )
    else()
        # On Linux/Unix, use ar with MRI script
        get_filename_component(LIB_NAME ${LIB_PATH} NAME)
        set(MRI_FILE ${CMAKE_CURRENT_BINARY_DIR}/${TARGET_NAME}_merge.mri)
        
        # Note: We need to use valid MRI script syntax
        # create libtarget.a
        # addlib libtarget.a
        # addlib libother.a
        # save
        # end
        
        add_custom_command(TARGET ${TARGET_NAME} POST_BUILD
            COMMAND echo "create $<TARGET_FILE:${TARGET_NAME}>" > ${MRI_FILE}
            COMMAND echo "addlib $<TARGET_FILE:${TARGET_NAME}>" >> ${MRI_FILE}
            COMMAND echo "addlib ${LIB_PATH}" >> ${MRI_FILE}
            COMMAND echo "save" >> ${MRI_FILE}
            COMMAND echo "end" >> ${MRI_FILE}
            COMMAND ar -M < ${MRI_FILE}
            COMMAND rm ${MRI_FILE}
        )
    endif()
endfunction()
