// Copyright (C) 2025 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

import QtQuick
import QtQuick.Controls
import QtTest

TestCase {
    name: "AbstractModelTest"
    id: test

    ListView {
        id: listview
        model: rustmodel
        reuseItems: false
        cacheBuffer: 10000
        delegate: Text {
            required property int value
            required property string decoration
            required property string display
            text: display
            color: decoration
        }
        width: 500
        height: 500
        visible: true
    }

    // Deliberately re-enters the model synchronously from beginInsertRows()
    property var rowCountsSeenOnInsert: []
    Connections {
        target: rustmodel
        function onRowsAboutToBeInserted() {
            test.rowCountsSeenOnInsert.push(rustmodel.rowCount())
        }
    }

    function check_sync() {
        listview.forceLayout()

        compare(listview.count, rustmodel.rowCount())
        for (let i = 0; i < rustmodel.rowCount(); ++i) {
            let index = rustmodel.index(i, 0)
            compare(listview.itemAtIndex(i).value, rustmodel.data(index, 2), "[" + i + "].value")
            compare(listview.itemAtIndex(i).decoration, rustmodel.data(index, 1), "[" + i + "].decoration")
            compare(listview.itemAtIndex(i).display, rustmodel.data(index, 0), "[" + i + "].display")
        }
    }

    function test_init() {
        rustmodel.resetItems()
        compare(rustmodel.rowCount(), 3)
        compare(rustmodel.columnCount(), 1)
        check_sync()
    }

    function test_push() {
        rustmodel.resetItems()
        rustmodel.pushFourItems()
        compare(rustmodel.rowCount(), 7)
        compare(rustmodel.columnCount(), 1)
        check_sync()
    }

    function test_remove() {
        rustmodel.resetItems()
        rustmodel.removeFirstTwoItems()
        compare(rustmodel.rowCount(), 1)
        compare(rustmodel.columnCount(), 1)
        let index = rustmodel.index(0, 0)
        compare(rustmodel.data(index, 2), 3)
        check_sync()
    }

    function test_pop() {
        rustmodel.resetItems()
        rustmodel.popTwoItems()
        compare(rustmodel.rowCount(), 1)
        compare(rustmodel.columnCount(), 1)
        check_sync()
    }

    function test_set() {
        rustmodel.resetItems()
        rustmodel.setTwoToNull()
        compare(rustmodel.rowCount(), 3)
        compare(rustmodel.columnCount(), 1)
        compare(rustmodel.data(rustmodel.index(1, 0), 2), 0)
        compare(rustmodel.data(rustmodel.index(2, 0), 2), 0)
        compare(rustmodel.data(rustmodel.index(1, 0), 1), "")
        compare(rustmodel.data(rustmodel.index(2, 0), 1), "")
        compare(rustmodel.data(rustmodel.index(1, 0), 0), "")
        compare(rustmodel.data(rustmodel.index(2, 0), 0), "")
        check_sync()
    }

    function test_insert() {
        rustmodel.resetItems()
        rustmodel.insertTwoAfterFirst()
        compare(rustmodel.rowCount(), 5)
        compare(rustmodel.columnCount(), 1)
        compare(rustmodel.data(rustmodel.index(1, 0), 2), 0)
        compare(rustmodel.data(rustmodel.index(2, 0), 2), 0)
        compare(rustmodel.data(rustmodel.index(1, 0), 1), "")
        compare(rustmodel.data(rustmodel.index(2, 0), 1), "")
        compare(rustmodel.data(rustmodel.index(1, 0), 0), "")
        compare(rustmodel.data(rustmodel.index(2, 0), 0), "")
        check_sync()
    }

    function test_push_reenters_row_count() {
        const before = rustmodel.rowCount()
        rowCountsSeenOnInsert = []
        rustmodel.pushFourItems()
        compare(rowCountsSeenOnInsert, [before, before + 1, before + 2, before + 3])
        compare(rustmodel.rowCount(), before + 4)
    }
}
