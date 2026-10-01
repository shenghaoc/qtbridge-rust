// Copyright (C) 2025 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

import QtQuick
import QtQuick.Controls
import QtTest

TestCase {
    name: "AbstractModelTest"
    id: test

    // Deliberately re-enters the model synchronously from beginInsertRows()
    property var rowCountsSeenOnInsert: []
    Connections {
        target: model
        function onRowsAboutToBeInserted() {
            test.rowCountsSeenOnInsert.push(model.rowCount())
        }
    }

    function test_base_functions() {
        compare(model.rowCount(), 3)
        compare(model.columnCount(), 5)
    }

    function test_insert_reenters_row_count() {
        rowCountsSeenOnInsert = []
        model.insertTopLevelRow()
        compare(rowCountsSeenOnInsert, [3])
        compare(model.rowCount(), 4)
    }
}
