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

    // Qt emits the *AboutTo* signals synchronously from within the
    // begin*() calls issued by the Rust model. Handlers that query the model
    // therefore re-enter the Rust proxy while the notification call is
    // still on the stack.
    property var reentrantRowCounts: []
    property bool observeReset: false
    Connections {
        target: rustmodel
        function onRowsAboutToBeInserted() {
            test.reentrantRowCounts.push(rustmodel.rowCount())
        }
        function onRowsAboutToBeRemoved() {
            test.reentrantRowCounts.push(rustmodel.rowCount())
        }
        function onModelAboutToBeReset() {
            // roleNames() is not invokable from QML; the ListView's
            // delegate model queries it from this same signal.
            if (test.observeReset)
                test.reentrantRowCounts.push(rustmodel.rowCount())
        }
        function onDataChanged() {
            test.reentrantRowCounts.push(rustmodel.rowCount())
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

    // These three tests deliberately avoid resetItems() and sort before
    // test_init() (QtTest runs test functions alphabetically): a reset also
    // re-enters the model (the ListView queries roleNames()), which would
    // otherwise hide failures of the notification under test.
    function test_basic_reentrant_insert() {
        const base = rustmodel.rowCount()
        reentrantRowCounts = []
        rustmodel.pushFourItems()
        // rowCount() as seen from rowsAboutToBeInserted: before each push
        compare(reentrantRowCounts, [base, base + 1, base + 2, base + 3])
        compare(rustmodel.rowCount(), base + 4)
        check_sync()
    }

    function test_basic_reentrant_remove() {
        rustmodel.pushFourItems()
        const base = rustmodel.rowCount()
        reentrantRowCounts = []
        rustmodel.removeFirstTwoItems()
        compare(reentrantRowCounts, [base, base - 1])
        compare(rustmodel.rowCount(), base - 2)
        check_sync()
    }

    function test_basic_reentrant_data_changed() {
        rustmodel.pushFourItems()
        const base = rustmodel.rowCount()
        reentrantRowCounts = []
        rustmodel.setTwoToNull()
        compare(reentrantRowCounts, [base, base])
        check_sync()
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

    function test_reentrant_on_reset() {
        rustmodel.resetItems()
        reentrantRowCounts = []
        observeReset = true
        rustmodel.resetItems()
        observeReset = false
        compare(reentrantRowCounts, [3])
        check_sync()
    }
}
