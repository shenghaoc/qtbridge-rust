// Copyright (C) 2025 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

import QtQuick
import QtQuick.Controls
import QtTest

TestCase {
    name: "AbstractModelTest"
    id: test

    // Qt emits the *AboutTo* signals synchronously from within the
    // begin*() calls issued by the Rust model, so these handlers re-enter
    // the Rust proxy while the notification call is still on the stack.
    property var rowCountsSeenOnInsert: []
    property var rowCountsSeenOnReset: []
    Connections {
        target: model
        function onRowsAboutToBeInserted() {
            test.rowCountsSeenOnInsert.push(model.rowCount())
        }
        function onModelAboutToBeReset() {
            test.rowCountsSeenOnReset.push(model.rowCount())
        }
    }

    function test_base_functions() {
        compare(model.rowCount(), 3)
        compare(model.columnCount(), 5)
    }

    function test_reentrant_row_count_on_insert() {
        rowCountsSeenOnInsert = []
        model.insertTopLevelRow()
        compare(rowCountsSeenOnInsert, [3])
        compare(model.rowCount(), 4)
    }

    function test_reentrant_row_count_on_reset() {
        rowCountsSeenOnReset = []
        model.resetTopLevelRows()
        compare(rowCountsSeenOnReset, [4])
        compare(model.rowCount(), 3)
    }
}
